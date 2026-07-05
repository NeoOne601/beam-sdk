#!/usr/bin/env bash
# Deploy the Axum backend to Fly.io free tier. Requires: flyctl installed and
# `fly auth login` already done (interactive, one-time). Reads secrets from
# your environment so nothing sensitive is committed.
#
#   DATABASE_URL=… REDIS_URL=… JWT_SECRET=… CORS_ALLOWED_ORIGINS=… \
#     ./deploy/deploy-backend.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

command -v fly >/dev/null || { echo "flyctl not installed — https://fly.io/docs/flyctl/install"; exit 1; }
: "${DATABASE_URL:?}" "${REDIS_URL:?}" "${JWT_SECRET:?}" "${CORS_ALLOWED_ORIGINS:?}"

# fly.toml lives in deploy/; copy it to root context for the deploy.
cp deploy/fly.toml ./fly.toml
trap 'rm -f ./fly.toml' EXIT

fly apps create ajna-verify 2>/dev/null || echo "app exists, continuing"
fly secrets set \
  DATABASE_URL="$DATABASE_URL" \
  REDIS_URL="$REDIS_URL" \
  JWT_SECRET="$JWT_SECRET" \
  CORS_ALLOWED_ORIGINS="$CORS_ALLOWED_ORIGINS"
fly deploy --ha=false

echo "→ verifying live health endpoint"
sleep 5
curl -fsS "https://ajna-verify.fly.dev/health" && echo " ✓ live"
