-- Beam Verify Backend — Initial Schema
-- Run with: psql -d beam_verify -f 001_initial.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE tenants (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL,
    api_key     TEXT NOT NULL UNIQUE,
    plan        TEXT NOT NULL DEFAULT 'startup',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE verification_results (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id),
    session_id          UUID NOT NULL,
    document_type       TEXT,
    issuing_country     TEXT,
    confidence          FLOAT,
    pqc_verified        BOOLEAN NOT NULL DEFAULT FALSE,
    pqc_public_key_hex  TEXT,
    fraud_signals       JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id),
    session_id      UUID,
    event_type      TEXT NOT NULL,
    outcome         TEXT NOT NULL,
    detail          JSONB,
    ip_address      INET,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE webhook_configs (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id),
    url         TEXT NOT NULL,
    events      TEXT[] NOT NULL,
    secret_hex  TEXT,
    active      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_tenant_time ON audit_logs(tenant_id, created_at DESC);
CREATE INDEX idx_verif_tenant_session ON verification_results(tenant_id, session_id);
