#!/usr/bin/env bash
# Deploy the React dashboard to Vercel. Requires: vercel CLI and `vercel login`
# (interactive, one-time). VITE_API_BASE points the SPA at your live backend.
#
#   VITE_API_BASE="https://ajna-verify.fly.dev" ./deploy/deploy-dashboard.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/dashboard"

command -v vercel >/dev/null || { echo "vercel CLI not installed — npm i -g vercel"; exit 1; }
: "${VITE_API_BASE:?set VITE_API_BASE to your backend URL}"

# Bake the backend URL into the production build.
echo "VITE_API_BASE=$VITE_API_BASE" > .env.production
npm ci || npm install
npm run build

vercel deploy --prod --yes
echo "✓ dashboard deployed (URL printed above)"
