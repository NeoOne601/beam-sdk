# Ajna — Live Deploy Checklist (your accounts)

Supabase is already provisioned by the assistant (project **Ajna**,
ref `wlskkisnuzlshsovnidb`, region ap-northeast-1): all 3 migrations applied,
5 tables live, demo tenant seeded (`api_key = ajna_live_sk_demo_0000`).

What's left needs your logins. Do these in order.

## 1. Supabase connection string (DATABASE_URL)
Supabase Dashboard → project **Ajna** → **Connect** (top bar) → **Session pooler**.
Copy the URI. It looks like:
```
postgresql://postgres.wlskkisnuzlshsovnidb:[YOUR-DB-PASSWORD]@aws-0-ap-northeast-1.pooler.supabase.com:5432/postgres
```
Replace `[YOUR-DB-PASSWORD]` with your project's DB password (set at project
creation; resettable under Settings → Database). Append `?sslmode=require`.
Use the **Session pooler** (port 5432), not Transaction (6543) — sqlx uses
prepared statements. This is your `DATABASE_URL`.

## 2. Upstash Redis (REDIS_URL) — free, no card
1. Sign up at https://upstash.com (GitHub login is fine).
2. Create Database → **Redis** → Free plan → any region.
3. On the DB page, copy the **`rediss://` URL** (TLS) from the "Redis" connect
   tab. That is your `REDIS_URL`.

## 3. Backend on Render — free, no card
1. Sign up at https://render.com (GitHub login).
2. Push this repo to GitHub (or use the existing `NeoOne601/beam-sdk`).
3. Render → **New → Blueprint** → pick this repo. It reads `deploy/render.yaml`
   and creates the `ajna-verify` web service (Docker, free plan).
4. In the service's **Environment**, set these (marked `sync:false` in the yaml):
   - `DATABASE_URL`  → from step 1
   - `REDIS_URL`     → from step 2
   - `JWT_SECRET`    → (assistant generated one; see chat)
   - `CORS_ALLOWED_ORIGINS` → your Vercel URL from step 4 (set after step 4,
     then redeploy — or use `*`-free explicit origin once known)
5. Deploy. When live, `https://ajna-verify.onrender.com/health` must return
   `{"status":"ok","db":"ok","redis":"ok"}`. (First hit after idle cold-starts ~1 min.)

## 4. Dashboard on Vercel
Two ways — pick one:
- **CLI:** `vercel login`, then from repo root `vercel deploy --prod`.
  `vercel.json` here builds `dashboard/` → `dashboard/dist`.
- **Git import:** Vercel → New Project → import the GitHub repo. Root dir `.`
  (vercel.json handles the dashboard build), framework "Other".
Then set env var `VITE_API_BASE = https://ajna-verify.onrender.com` (Production)
and redeploy. Put the resulting `https://<project>.vercel.app` into the
backend's `CORS_ALLOWED_ORIGINS` (step 3.4) and redeploy the backend.

## 5. Verify end-to-end
- Open the Vercel URL → **Audit Log** tab → backend URL + `ajna_live_sk_demo_0000`
  → Load entries / Verify chain hit the live backend.
- Point a mobile sample (examples/*/README.md) at the Render URL, run on your
  device with your real ID + face; the transaction appears on the dashboard.
