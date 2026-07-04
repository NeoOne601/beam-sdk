#!/bin/bash
echo "## Part A — confirm repo state and recent history"
git log --oneline -20
git status
git branch --show-current

echo "## Part B — check Gap Prompts 1-4 (production fixes)"
echo "--- Gap 1: Ed25519 verifier ---"
test -f backend/src/crypto/ed25519_verifier.rs && echo "FILE EXISTS" || echo "MISSING"
grep -c "pub fn verify_ed25519" backend/src/crypto/ed25519_verifier.rs 2>/dev/null || echo "0"

echo "--- Gap 1: dispatch in verify.rs ---"
grep -n "scan_result.algo\|\"ed25519\" =>" backend/src/routes/verify.rs 2>/dev/null || echo "NOT FOUND"

echo "--- Gap 1: registered in crypto/mod.rs ---"
grep -n "pub mod ed25519_verifier" backend/src/crypto/mod.rs 2>/dev/null || echo "NOT FOUND"

echo "--- Gap 3: JWS module ---"
test -f crates/beam-crypto/src/jws.rs && echo "FILE EXISTS" || echo "MISSING"
grep -c "pub fn produce_jws" crates/beam-crypto/src/jws.rs 2>/dev/null || echo "0"
grep -n "jws_token" core/src/result.rs 2>/dev/null || echo "NOT FOUND IN result.rs"
grep -n "pub mod jws" crates/beam-crypto/src/lib.rs 2>/dev/null || echo "NOT FOUND"

echo "--- Gap 4: ADR-002 ---"
test -f docs/adr/ADR-002-redis-before-enrollment.md && echo "FILE EXISTS" || echo "MISSING"

echo "--- Gap 2 and 4: CI status at those commits ---"
gh run list --limit 10 || echo "gh command failed"

echo "## Part C — check Session 3 (React Native wrapper)"
echo "--- Session 3: package exists ---"
test -d packages/react-native && echo "DIR EXISTS" || echo "MISSING"
test -f packages/react-native/package.json && echo "package.json EXISTS" || echo "MISSING"
test -f packages/react-native/src/BeamCamera.tsx && echo "BeamCamera.tsx EXISTS" || echo "MISSING"
test -f packages/react-native/src/BeamNativeModule.ts && echo "BeamNativeModule.ts EXISTS" || echo "MISSING"
test -f packages/react-native/src/index.ts && echo "index.ts EXISTS" || echo "MISSING"

echo "--- Session 3: npm publish status ---"
grep -n '"version"' packages/react-native/package.json 2>/dev/null || echo "N/A - package.json missing"
npm view @beam/react-native version 2>&1 | head -3

echo "## Part D — check Session 4 Corrected v2 (tenant policy, onboarding, verifications list)"
echo "--- File 1: tenant_policies migration ---"
test -f backend/src/db/migrations/003_tenant_policies.sql && echo "FILE EXISTS" || echo "MISSING"
grep -n "CREATE TABLE tenant_policies" backend/src/db/migrations/003_tenant_policies.sql 2>/dev/null

echo "--- File 2: session.rs policy extraction ---"
grep -n "TenantContext\|SessionPolicy\|tenant_policies" backend/src/routes/session.rs 2>/dev/null || echo "NOT MODIFIED — still original version"

echo "--- File 3: onboard.rs ---"
test -f backend/src/routes/onboard.rs && echo "FILE EXISTS" || echo "MISSING"
grep -c "pub async fn register" backend/src/routes/onboard.rs 2>/dev/null || echo "0"

echo "--- File 4: routes/mod.rs wiring ---"
grep -n "pub mod onboard\|onboard_router\|list_verifications" backend/src/routes/mod.rs 2>/dev/null || echo "NOT WIRED"

echo "--- File 5: verify.rs list_verifications ---"
grep -c "pub async fn list_verifications" backend/src/routes/verify.rs 2>/dev/null || echo "0"

echo "--- .sqlx cache freshness ---"
ls -la .sqlx/ 2>/dev/null | tail -10
git log -1 --format="%H %ci" -- .sqlx/ 2>/dev/null || echo "No commits touching .sqlx/"

echo "## Part E — check main.rs router wiring for onboard_router"
grep -n "onboard_router\|health_router" backend/src/main.rs 2>/dev/null || echo "NOT FOUND"

echo "## Part F — actual build and test status right now"
cargo build --workspace 2>&1 | tail -30
echo "BUILD EXIT CODE: ${PIPESTATUS[0]}"

echo "## Part G — dashboard directory state"
test -d dashboard && echo "dashboard/ EXISTS" || echo "MISSING"
ls -la dashboard/ 2>/dev/null
test -f dashboard/index.html && echo "index.html present (known placeholder from earlier repomix)" || true
test -d dashboard/app && echo "Next.js app/ directory present — Session 5 may have partially run" || echo "No Next.js structure — Session 5 has not started"
