#!/usr/bin/env bash
# Apply the Ajna schema to any Postgres reachable via $DATABASE_URL
# (local, Neon, or Supabase). Idempotent-ish: migrations use IF NOT EXISTS
# where they can; re-running 001 on a populated DB will error harmlessly.
#
#   DATABASE_URL="postgres://…" ./deploy/provision-db.sh
set -euo pipefail

: "${DATABASE_URL:?set DATABASE_URL to your Postgres connection string}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIG="$ROOT/backend/src/db/migrations"

for f in 001_initial.sql 002_trusted_keys.sql 003_audit_chain.sql; do
  echo "→ applying $f"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$MIG/$f"
done
echo "✓ schema applied"
