# Ajna — $0 Deployment Runbook

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
