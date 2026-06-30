# ADR-002: Redis key cache must be built before device enrollment opens

## Status
Accepted

## Context
POST /v1/verify currently performs a SELECT on trusted_public_keys for every
verification request. At low volume this is acceptable. Once POST /v1/device/enroll
is opened to customers, devices will register public keys. As enrollment scales,
every verification hits the database for a key lookup. Retrofitting a Redis cache
onto a live enrollment pipeline requires solving cache invalidation against an
already-running system with real customer keys — a non-trivial distributed systems
problem that cannot be safely done without downtime.

## Decision
The Redis caching layer for trusted_public_keys lookups MUST be implemented
and deployed before POST /v1/device/enroll is opened to any customer.

Implementation order:
1. Add Upstash Redis (free tier) connection to backend
2. Implement key lookup with Redis-aside cache: check Redis first, fall back to
   Postgres on miss, write to Redis with TTL=300s on miss
3. Implement cache invalidation on key enrollment and key revocation
4. Only after Redis is live and invalidation is tested: open device enrollment

## Consequences
Any engineer or AI session implementing device enrollment MUST check that
Redis caching for key lookups is already live before writing the enrollment endpoint.
If it is not live, implement Redis first.

## Related
- Phase 4 in project_roadmap.md
- POST /v1/device/enroll (not yet implemented as of this ADR)
