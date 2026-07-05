-- Ajna Verify Backend — Trusted Public Key Registry
-- Migration 002: VR-2 (Security) — Trusted key registration replacing client-supplied keys.
--
-- Architectural decision (README.md §VR-2):
--   Strategy Pattern with three concrete implementations:
--     - tenant:  one pre-registered pubkey per tenant (KEY_PROVIDER_STRATEGY=tenant)
--     - device:  per-device key via device_id claim   (KEY_PROVIDER_STRATEGY=device)
--     - model:   per-model-version key                (KEY_PROVIDER_STRATEGY=model)
--
--   The key_id column is the lookup key for all strategies. Its meaning depends
--   on the active strategy:
--     tenant  → set to tenant_id::text (implicit; no extra claim needed)
--     device  → set to device_id (supplied in request body alongside signature)
--     model   → set to model_id + "/" + model_version (supplied in request body)
--
-- Run with: psql -d ajna_verify -f 002_trusted_keys.sql

CREATE TABLE trusted_public_keys (
    id          UUID        PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    -- Human-readable identifier for the key.
    -- tenant strategy : equals tenant_id::text
    -- device strategy : equals device_id string
    -- model strategy  : equals "model_id/model_version"
    key_id      TEXT        NOT NULL,
    -- Raw ML-DSA (Dilithium-3) public key bytes — 1952 bytes for Level 3.
    public_key  BYTEA       NOT NULL,
    -- Algorithm label for forward-compatibility when we add Level 5 or other schemes.
    algorithm   TEXT        NOT NULL DEFAULT 'ML-DSA-65',
    active      BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Populated when a key is revoked; revoked keys are rejected at verification time.
    revoked_at  TIMESTAMPTZ,
    -- Optional descriptive label for operator dashboards.
    label       TEXT,

    -- Ensure (tenant_id, key_id) is unique so lookups are unambiguous.
    UNIQUE (tenant_id, key_id)
);

CREATE INDEX idx_tpk_tenant_active ON trusted_public_keys(tenant_id, active)
    WHERE active = TRUE AND revoked_at IS NULL;

-- Verification results now store key_id for audit trail.
ALTER TABLE verification_results
    ADD COLUMN IF NOT EXISTS key_id TEXT;

COMMENT ON TABLE trusted_public_keys IS
    'Registry of pre-registered ML-DSA public keys. Backend verifies signatures '
    'against keys in this table only — client-supplied public keys are never trusted. '
    'See README.md §VR-2 and KEY_PROVIDER_STRATEGY env var.';
