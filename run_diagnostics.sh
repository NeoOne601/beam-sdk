#!/bin/bash

echo "## Investigation A — SQLX_OFFLINE discrepancy"
echo "--- CI workflow env vars ---"
grep -rn "SQLX_OFFLINE\|DATABASE_URL" .github/workflows/

echo "--- sqlx crate feature flags in use ---"
grep -n "sqlx" backend/Cargo.toml core/Cargo.toml crates/beam-crypto/Cargo.toml 2>/dev/null

echo "--- .sqlx cache directory state ---"
ls -la .sqlx/
git log -3 --format="%H %ci %s" -- .sqlx/

echo "--- local env check ---"
echo "SQLX_OFFLINE is currently: ${SQLX_OFFLINE:-UNSET}"
echo "DATABASE_URL is currently: ${DATABASE_URL:+SET (hidden)}${DATABASE_URL:-UNSET}"

echo "--- local build WITHOUT SQLX_OFFLINE set ---"
env -u SQLX_OFFLINE cargo build --workspace 2>&1 | tail -20
echo "EXIT CODE: ${PIPESTATUS[0]}"

echo "--- local build WITH SQLX_OFFLINE=true explicitly set ---"
SQLX_OFFLINE=true cargo build --workspace 2>&1 | tail -20
echo "EXIT CODE: ${PIPESTATUS[0]}"

echo "## Investigation B — audit commit b0a9efb"
echo "--- git show stat ---"
git show b0a9efb --stat

echo "--- git show Cargo.toml ---"
git show b0a9efb -- '*/Cargo.toml'

echo "--- git show signers ---"
git show b0a9efb -- 'crates/beam-crypto/src/signers/'

echo "--- grep hybrid ---"
grep -rn "HybridSigner\|hybrid_signer\|hybrid_stub" --include="*.rs" .

echo "--- grep required_algo ---"
grep -n "required_algo" backend/ core/ crates/ -r 2>/dev/null || echo "NOT FOUND"

echo "## Investigation C — quick check on the RUSTSEC advisory commit"
git show 3e95767 --stat
