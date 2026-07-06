# Ajna Cloud Deploy — $0 Infrastructure Plan

## Architecture (all free tier)

```
[iPhone/Android app] ──► Backend (Rust/Axum container)         ── Render free web service
                              │  DATABASE_URL ────────────────► Supabase Postgres (connector ✓)
                              │  REDIS_URL ───────────────────► Upstash Redis (needs signup)
                              │  JWT_SECRET / CORS env
[Browser] ───────────────► Dashboard (Vite SPA) ─────────────── Vercel (connector ✓)
                              VITE_API_BASE → backend URL
```

Decoupling: identical binary everywhere; only env vars change
(backend/src/db/pool.rs reads DB_* + DATABASE_URL at boot).

## Hosting decision (backend)
- **Render free web service — chosen.** No CLI required (Blueprint from a
  GitHub repo, deploy/render.yaml already written), free tier is $0 with
  sleep-after-idle. Acceptable for a demo.
- Fly.io — rejected: requires a payment card on file (not "completely free").
- Koyeb — viable fallback (free instance, GitHub deploy), same pattern.
- Redis: Upstash free (10k cmd/day) — nonce store + rate limiter.

## Steps
- [ ] 1. Supabase (connector): discover project → apply migrations 001/002/003
        → verify tables → seed demo tenant + trusted key → record project ref
        + connection-string TEMPLATE (password stays with user).
- [ ] 2. Vercel (connector): deploy dashboard/ to production. VITE_API_BASE
        injected once backend URL exists (redeploy is one call).
- [ ] 3. Backend on Render: needs USER signup + GitHub repo connect →
        STOP and give numbered instructions (per goal rules).
- [ ] 4. Upstash Redis: needs USER signup → included in same instructions.
- [ ] 5. After user provides: backend env vars set in Render dashboard,
        /health returns {"status":"ok","db":"ok","redis":"ok"}, dashboard
        redeployed with final VITE_API_BASE, audit tab verified against
        live backend.

## Credentials policy
Secrets (Supabase DB password, Upstash URL) are never pasted into chat: user
puts them in Render's env-var UI directly; local testing uses a gitignored
.env file.
