# Lessons & Anti-patterns

> Append-only. Review at the start of every cycle (CLAUDE.md §19).

- **[2026-07-12] 8 GB M1 discipline:** every cargo command is `SQLX_OFFLINE=true ... -j 2`;
  never parallel heavy builds; Docker mem caps (Postgres 256m, Redis 128m).
- **[2026-07-12] code-review-graph hook is broken/stale** — its per-edit errors are
  noise; do not chase them, do not rely on the graph until rebuilt.
- **[2026-07-12] Never commit `dump.rdb`** (local Redis state) or stray root-level
  artifacts (`CLAUDE(1).md`, PDFs) — keep commits surgical.
- **[2026-07-12] npm deps are a Human Gate (§13)** except the three §15 enumerates
  (react-router-dom, recharts, lucide-react). Anything else: ask first.
- **[2026-07-12] Dashboard-only changes:** the verification gate is `npm run build`
  (tsc + vite); full `cargo test --release -j 2` only when Rust is touched — running
  a 30-min release build for a CSS change wastes the cycle.
