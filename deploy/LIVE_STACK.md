# Ajna — Live Stack Reference

Deployed $0 infrastructure. All URLs verified live end-to-end.

## Live endpoints
| Component | URL |
|---|---|
| Backend API (Render, free) | https://ajna-verify.onrender.com |
| Health | https://ajna-verify.onrender.com/health → `{"status":"ok","db":"ok","redis":"ok"}` |
| Dashboard (Vercel) | https://ajna-platform.vercel.app |

Demo API key (Audit tab / mobile app): `ajna_live_sk_demo_0000`

## Database — Supabase (connection string reference)
- Project: **Ajna**, ref `wlskkisnuzlshsovnidb`, region ap-northeast-1, Postgres 17.
- Migrations 001/002/003 applied; 5 tables live; demo tenant + trusted key seeded.
- `DATABASE_URL` (Session pooler, IPv4 — password held by owner, not stored here):
  ```
  postgresql://postgres.wlskkisnuzlshsovnidb:[DB_PASSWORD]@aws-0-ap-northeast-1.pooler.supabase.com:5432/postgres?sslmode=require
  ```

## Hosting decisions
- **Backend + Redis: Render free tier.** One Blueprint (`deploy/render.yaml`)
  provisions the Axum web service *and* a free Key Value (Redis) instance —
  `REDIS_URL` auto-wired via `fromService`, so **no Upstash account was needed**.
  Free web service sleeps after ~15 min idle; first request cold-starts ~50s.
- Fly.io rejected (requires a card). Cloudflare Pages excluded per requirement.

## Backend env (Render, service `ajna-verify`)
| Var | Source |
|---|---|
| `DATABASE_URL` | Supabase Session pooler (set in dashboard) |
| `REDIS_URL` | auto-wired from the `ajna-redis` Key Value service |
| `JWT_SECRET` | 32-byte hex (set in dashboard; not needed for API-key auth) |
| `CORS_ALLOWED_ORIGINS` | `https://ajna-platform.vercel.app` (in render.yaml) |
| `DB_REQUIRE_TLS` | `true` — requires `sqlx` `tls-rustls` feature (now enabled) |

## Dashboard (Vercel project `ajna-platform`, repo NeoOne601/ajna-platform)
- `VITE_API_BASE=https://ajna-verify.onrender.com` baked in at build
  (`dashboard/.env.production`); Audit tab defaults to the live backend.

## Verified (browser-equivalent)
- `/health` = ok · dashboard HTTP 200 · CORS allows the Vercel origin ·
  `GET /v1/audit` returns entries · `verify-chain` = `valid:true` ·
  real ML-DSA-65-signed verifications persist to Supabase.

## Run it with your real ID
1. Open https://ajna-platform.vercel.app → Audit Log → key `ajna_live_sk_demo_0000`.
2. Build a mobile sample (examples/*/README.md), point `BACKEND_URL` at
   https://ajna-verify.onrender.com, run on your device, scan your ID + face.
3. The verification transaction appears in the Audit tab.
