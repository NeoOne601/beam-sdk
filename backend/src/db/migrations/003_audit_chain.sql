-- 003_audit_chain.sql
-- SOC2 Type 2: make audit_logs tamper-evident and append-only.
--
--   1. seq        — monotonically increasing position for chain ordering.
--   2. prev_hash  — entry_hash of the previous entry in this tenant's chain
--                   ("genesis" for the first entry).
--   3. entry_hash — SHA-256 over (prev_hash | tenant | session | event_type
--                   | outcome | detail), computed in the application
--                   (backend/src/models/audit_log.rs).
--   4. Trigger    — UPDATE and DELETE on audit_logs are rejected at the
--                   database level. Retention is handled by partition
--                   archival, never row deletion.

ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS seq BIGSERIAL;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS prev_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS entry_hash TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_audit_tenant_seq ON audit_logs(tenant_id, seq DESC);

CREATE OR REPLACE FUNCTION audit_logs_block_mutation() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'audit_logs is append-only (SOC2 tamper-evidence): % rejected', TG_OP;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS audit_logs_append_only ON audit_logs;
CREATE TRIGGER audit_logs_append_only
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION audit_logs_block_mutation();
