// backend/src/models/audit_log.rs
//
// SOC2 Type 2 tamper-evident audit trail.
//
// Every entry is hash-chained per tenant: entry_hash = SHA-256 over
// (prev_hash | tenant_id | session_id | event_type | outcome | detail).
// Combined with the database-level append-only trigger (migration 003),
// any retroactive edit breaks the chain and is detectable by
// GET /v1/audit/verify-chain.
//
// Inserts use runtime-checked sqlx queries deliberately: compile-time
// query! macros would require regenerating the .sqlx offline cache against
// a live database for every schema change.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// prev_hash sentinel for the first entry in a tenant's chain.
pub const GENESIS_HASH: &str = "genesis";

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub session_id: Option<Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: time::OffsetDateTime,
}

/// One row of the chain as loaded for verification.
#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub seq: i64,
    pub tenant_id: Uuid,
    pub session_id: Option<Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: serde_json::Value,
    pub prev_hash: String,
    pub entry_hash: String,
}

/// Result of walking a tenant's audit chain.
#[derive(Debug, Serialize)]
pub struct ChainReport {
    pub valid: bool,
    pub entries_checked: i64,
    /// Rows written before migration 003 (no hash); reported, not verified.
    pub legacy_entries_skipped: i64,
    /// seq of the first entry whose hash or linkage fails, if any.
    pub first_broken_seq: Option<i64>,
}

/// Deterministic entry hash. Field order and separator are part of the
/// audit chain format — never change without a chain-epoch migration.
pub fn compute_entry_hash(
    prev_hash: &str,
    tenant_id: Uuid,
    session_id: Option<Uuid>,
    event_type: &str,
    outcome: &str,
    detail: &serde_json::Value,
) -> String {
    let session = session_id.map(|s| s.to_string()).unwrap_or_default();
    let material = format!("{prev_hash}|{tenant_id}|{session}|{event_type}|{outcome}|{detail}");
    hex::encode(Sha256::digest(material.as_bytes()))
}

/// Verify linkage and per-entry hashes over a tenant's chain, ordered by
/// ascending seq. Pure function — the route handler feeds it DB rows.
pub fn verify_chain_entries(entries: &[ChainEntry]) -> ChainReport {
    let mut expected_prev = GENESIS_HASH.to_owned();
    let mut checked = 0i64;
    let mut legacy = 0i64;
    let mut first_broken = None;

    for entry in entries {
        // Rows predating migration 003 carry empty hashes; they cannot be
        // verified retroactively and are reported separately.
        if entry.entry_hash.is_empty() {
            legacy += 1;
            continue;
        }
        checked += 1;

        let recomputed = compute_entry_hash(
            &entry.prev_hash,
            entry.tenant_id,
            entry.session_id,
            &entry.event_type,
            &entry.outcome,
            &entry.detail,
        );
        let linked = entry.prev_hash == expected_prev;
        if recomputed != entry.entry_hash || !linked {
            first_broken = Some(entry.seq);
            break;
        }
        expected_prev = entry.entry_hash.clone();
    }

    ChainReport {
        valid: first_broken.is_none(),
        entries_checked: checked,
        legacy_entries_skipped: legacy,
        first_broken_seq: first_broken,
    }
}

/// Append one entry to the tenant's tamper-evident chain.
pub async fn record_audit_event(
    pool: &sqlx::PgPool,
    tenant_id: Uuid,
    session_id: Option<Uuid>,
    event_type: &str,
    outcome: &str,
    detail: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // ponytail: per-tenant advisory lock serializes chain appends; audit
    // write rates are low — switch to a queued writer if this ever contends.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;

    // Link to the latest *chained* entry, skipping unhashed rows. Other event
    // types (e.g. nonce_created) are written unhashed by their routes and get
    // interleaved between verifications by seq; selecting the latest row
    // outright would reset a live chain to genesis whenever such a row is the
    // head, breaking linkage. verify_chain_entries skips the same unhashed rows.
    let prev_hash: Option<String> = sqlx::query_scalar(
        "SELECT entry_hash FROM audit_logs \
         WHERE tenant_id = $1 AND entry_hash <> '' ORDER BY seq DESC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?;
    let prev_hash = prev_hash.unwrap_or_else(|| GENESIS_HASH.to_owned());

    let entry_hash = compute_entry_hash(
        &prev_hash, tenant_id, session_id, event_type, outcome, &detail,
    );

    sqlx::query(
        r#"
        INSERT INTO audit_logs
            (id, tenant_id, session_id, event_type, outcome, detail, prev_hash, entry_hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id)
    .bind(session_id)
    .bind(event_type)
    .bind(outcome)
    .bind(detail)
    .bind(&prev_hash)
    .bind(&entry_hash)
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seq: i64, tenant: Uuid, prev_hash: &str, outcome: &str) -> ChainEntry {
        let detail = serde_json::json!({ "n": seq });
        let entry_hash =
            compute_entry_hash(prev_hash, tenant, None, "verification", outcome, &detail);
        ChainEntry {
            seq,
            tenant_id: tenant,
            session_id: None,
            event_type: "verification".into(),
            outcome: outcome.into(),
            detail,
            prev_hash: prev_hash.into(),
            entry_hash,
        }
    }

    fn chain_of(tenant: Uuid, outcomes: &[&str]) -> Vec<ChainEntry> {
        let mut entries = Vec::new();
        let mut prev = GENESIS_HASH.to_owned();
        for (i, outcome) in outcomes.iter().enumerate() {
            let e = entry(i as i64 + 1, tenant, &prev, outcome);
            prev = e.entry_hash.clone();
            entries.push(e);
        }
        entries
    }

    #[test]
    fn entry_hash_is_deterministic_and_input_sensitive() {
        let tenant = Uuid::nil();
        let detail = serde_json::json!({ "k": "v" });
        let a = compute_entry_hash(
            GENESIS_HASH,
            tenant,
            None,
            "verification",
            "success",
            &detail,
        );
        let b = compute_entry_hash(
            GENESIS_HASH,
            tenant,
            None,
            "verification",
            "success",
            &detail,
        );
        let c = compute_entry_hash(
            GENESIS_HASH,
            tenant,
            None,
            "verification",
            "failure",
            &detail,
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64); // SHA-256 hex
    }

    #[test]
    fn intact_chain_verifies() {
        let chain = chain_of(Uuid::nil(), &["success", "success", "failure", "success"]);
        let report = verify_chain_entries(&chain);
        assert!(report.valid);
        assert_eq!(report.entries_checked, 4);
        assert_eq!(report.legacy_entries_skipped, 0);
    }

    #[test]
    fn tampered_middle_entry_breaks_the_chain_at_that_seq() {
        let mut chain = chain_of(Uuid::nil(), &["success", "failure", "success"]);
        // Retroactively "fix" the failed verification without re-hashing.
        chain[1].outcome = "success".into();
        let report = verify_chain_entries(&chain);
        assert!(!report.valid);
        assert_eq!(report.first_broken_seq, Some(2));
    }

    #[test]
    fn relinked_forgery_is_caught_by_linkage_check() {
        let mut chain = chain_of(Uuid::nil(), &["success", "failure", "success"]);
        // Forge entry 2 completely (valid self-hash, wrong linkage downstream).
        let forged = entry(2, Uuid::nil(), &chain[0].entry_hash, "success");
        chain[1] = forged;
        let report = verify_chain_entries(&chain);
        assert!(
            !report.valid,
            "entry 3 must no longer link to forged entry 2"
        );
        assert_eq!(report.first_broken_seq, Some(3));
    }

    #[test]
    fn legacy_unhashed_rows_are_skipped_not_failed() {
        let tenant = Uuid::nil();
        let mut chain = chain_of(tenant, &["success", "success"]);
        chain.insert(
            0,
            ChainEntry {
                seq: 0,
                tenant_id: tenant,
                session_id: None,
                event_type: "verification".into(),
                outcome: "success".into(),
                detail: serde_json::json!({}),
                prev_hash: String::new(),
                entry_hash: String::new(),
            },
        );
        let report = verify_chain_entries(&chain);
        assert!(report.valid);
        assert_eq!(report.entries_checked, 2);
        assert_eq!(report.legacy_entries_skipped, 1);
    }

    /// Regression (found via the live demo): unhashed rows (e.g. nonce_created)
    /// interleaved *between* hashed verification entries by seq must not break
    /// the chain. Each hashed entry links to the previous HASHED entry, and the
    /// writer's prev_hash lookup skips unhashed rows — mirrored here.
    fn unhashed_row(seq: i64, tenant: Uuid) -> ChainEntry {
        ChainEntry {
            seq,
            tenant_id: tenant,
            session_id: None,
            event_type: "nonce_created".into(),
            outcome: "success".into(),
            detail: serde_json::json!({}),
            prev_hash: String::new(),
            entry_hash: String::new(),
        }
    }

    #[test]
    fn hashed_entries_interleaved_with_unhashed_rows_stay_valid() {
        let tenant = Uuid::nil();
        // Timeline by seq: nonce, verify, nonce, verify — the real DB pattern.
        let mut prev = GENESIS_HASH.to_owned();
        let v2 = entry(2, tenant, &prev, "success");
        prev = v2.entry_hash.clone();
        let v4 = entry(4, tenant, &prev, "success"); // links to v2, NOT to seq-3 nonce
        let chain = vec![
            unhashed_row(1, tenant),
            v2,
            unhashed_row(3, tenant),
            v4,
        ];
        let report = verify_chain_entries(&chain);
        assert!(report.valid, "interleaved unhashed rows must not break linkage");
        assert_eq!(report.entries_checked, 2);
        assert_eq!(report.legacy_entries_skipped, 2);
    }
}
