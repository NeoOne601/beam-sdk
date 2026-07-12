# Ajna — $0 Deployment Runbook

> **Verified live run (local):** the full stack was brought up and exercised
> end-to-end — Postgres + Redis + the Axum backend booting against the tuned
> pool (`max_connections=5`), `/health` → `{"status":"ok","db":"ok","redis":"ok"}`,
> a real Ed25519-signed `/v1/verify` returning `verified:true` with an
> **ML-DSA-65 server attestation**, the country-rules + NQM envelopes applied,
> the transaction persisted to the tamper-evident SHA-256 audit chain
> (`/v1/audit/verify-chain` → `valid:true`), and the React dashboard rendering
> that live transaction + chain-integrity status. Reproduce it below.
>
> Publishing to a *public* free-tier URL (Fly/Render/Vercel + Neon) needs your
> accounts and one-time interactive logins — the scripts below do it.

## Reproduce the verified local run
```bash
brew install postgresql@16 redis
export LC_ALL=en_US.UTF-8
brew services start postgresql@16 && brew services start redis
createdb ajna_verify 2>/dev/null; psql -d postgres -c "CREATE ROLE ajna LOGIN PASSWORD 'ajna' SUPERUSER" 2>/dev/null
export DATABASE_URL="postgres://ajna:ajna@localhost:5432/ajna_verify?sslmode=disable"
./deploy/provision-db.sh
# seed a demo tenant + trusted key (see backend/examples/sign_demo.rs header)
# ALLOW_UNREGISTERED_ED25519_KEYS=true is DEMO-ONLY (VR-2): sign_demo signs with
# a fresh ephemeral ed25519 key, which is not in trusted_public_keys. Production
# deployments must omit this flag — verification then fails closed against
# unregistered keys, and such demo verifications are marked
# "key_trust":"client-supplied-demo" in the audit chain.
DB_REQUIRE_TLS=false REDIS_URL=redis://127.0.0.1:6379 \
  ALLOW_UNREGISTERED_ED25519_KEYS=true \
  CORS_ALLOWED_ORIGINS=http://localhost:5173 ./target/release/ajna-verify-backend &
curl -s localhost:8080/health
# drive a signed verification:
SID=$(uuidgen); TS=$(date +%s)
NONCE=$(curl -s -XPOST localhost:8080/v1/nonce -H "X-Api-Key: ajna_live_sk_demo_0000" \
  -H 'Content-Type: application/json' -d "{\"session_id\":\"$SID\"}" | jq -r .nonce)
cargo run --example sign_demo -- "$NONCE" "$SID" "$TS" \
  | curl -s -XPOST localhost:8080/v1/verify -H "X-Api-Key: ajna_live_sk_demo_0000" \
    -H 'Content-Type: application/json' -d @- | jq .verified   # → true
```


Everything here runs on free tiers. Config files live beside this doc
(`deploy/fly.toml`, `deploy/render.yaml`, `dashboard/vercel.json`); the actual
deploy commands need **your** accounts and interactive logins, so they can't be
run from an autonomous shell — run them yourself in this order.

Architecture is decoupled by env vars: the same backend binary runs locally,
on Fly, or on Render, pointed at local Postgres or serverless Neon/Supabase —
only environment variables change (see `backend/src/db/pool.rs`).

## 0. Local smoke test (Docker)

```bash
docker compose up --build
# API       → http://localhost:8080/health   → {"status":"ok"}
# Dashboard → http://localhost:5173
```
All four services are capped at `mem_limit: 256m`.

## 1. Database — Neon (or Supabase) free Postgres

1. Create a project at neon.tech (or supabase.com). Copy the pooled connection
   string. It already includes `?sslmode=require`.
2. Apply the schema (migrations are plain SQL):
   ```bash
   psql "$DATABASE_URL" -f backend/src/db/migrations/001_initial.sql
   psql "$DATABASE_URL" -f backend/src/db/migrations/002_trusted_keys.sql
   psql "$DATABASE_URL" -f backend/src/db/migrations/003_audit_chain.sql
   ```
> Redis: use Upstash's free tier for `REDIS_URL` (nonce store + rate limiter).

## 2. Backend — Fly.io  (or Render)

**Fly:**
```bash
fly launch --no-deploy --copy-config      # uses deploy/fly.toml
fly secrets set \
  DATABASE_URL="postgres://…neon…/db?sslmode=require" \
  REDIS_URL="rediss://…upstash…" \
  JWT_SECRET="$(openssl rand -hex 32)" \
  CORS_ALLOWED_ORIGINS="https://<your-dashboard>.vercel.app"
fly deploy
curl https://ajna-verify.fly.dev/health          # → {"status":"ok"}
```

**Render:** New → Blueprint → this repo (`deploy/render.yaml`). Set
`DATABASE_URL`, `REDIS_URL`, `JWT_SECRET`, `CORS_ALLOWED_ORIGINS` as secret env
vars. First request after idle cold-starts (~30s on free tier).

## 3. Dashboard — Vercel  (or Cloudflare Pages)

```bash
cd dashboard
vercel                                   # uses dashboard/vercel.json
vercel env add VITE_API_BASE production  # → your backend URL, e.g. https://ajna-verify.fly.dev
vercel --prod
```
Cloudflare Pages equivalent: framework **Vite**, build `npm run build`, output
`dist`, root directory `dashboard`.

## 4. Verify live

```bash
curl https://ajna-verify.fly.dev/health
# Open the Vercel URL → Audit Log tab → enter backend URL + API key →
# "Load recent entries" and "Verify chain" hit the live backend.
```

## Scaling to enterprise (no code change)

Raise `DB_MAX_CONNECTIONS`, point `DATABASE_URL` at a dedicated Postgres, bump
the Fly `[[vm]]` memory, set `min_machines_running > 0` to drop cold starts.
The connection pool (`backend/src/db/pool.rs`) reads all of it from env.
