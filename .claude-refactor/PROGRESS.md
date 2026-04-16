# Koentji Refactor — Progress

- Plan: `/root/.claude/plans/use-razif-coding-style-audit-current-velvet-lampson.md`
- Started: 2026-04-17
- Current phase: 0
- Next commit: 0.1

## Checklist

### Phase 0 — safety net
- [ ] 0.1  test: add integration harness and domain-test helpers
- [ ] 0.2  fix: bind custom date-range parameters in dashboard stats query
- [ ] 0.3  fix: decrement rate limit atomically on /v1/auth
- [ ] 0.4  chore: drop stale agAuth/ references from docs
- [ ] 0.5  tec: make check aggregates fmt + clippy + test; CI runs it

### Phase 1 — domain extraction
- [ ] 1.1  tec: introduce domain module skeleton and value objects
- [ ] 1.2  tec: extract IssuedKey aggregate with lifecycle verbs
- [ ] 1.3  tec: define AuthDenialReason enum with en/id mapping at HTTP edge
- [ ] 1.4  tec: introduce IssuedKeyRepository port + Postgres adapter
- [ ] 1.5  tec: hide Moka auth cache behind AuthCachePort
- [ ] 1.6  feat: route /v1/auth through AuthenticateApiKey use case
- [ ] 1.7  test: cover IssuedKey.authorize across all denial reasons
- [ ] 1.8  test: integration tests for Postgres IssuedKeyRepository

### Phase 2 — admin verbs
- [ ] 2.1  feat: issuing a key emits KeyIssued and returns an IssuedKey
- [ ] 2.2  feat: revoking a key emits KeyRevoked and invalidates auth cache
- [ ] 2.3  feat: reassigning a device / resetting rate limit / extending expiration as domain commands
- [ ] 2.4  fix: device reassignment also evicts the prior auth-cache entry
- [ ] 2.5  test: cover key-management commands

### Phase 3 — schema hardening
- [ ] 3.1  feat: enforce UNIQUE(key, device_id)
- [ ] 3.2  feat: index authentication_keys on (key, device_id)
- [ ] 3.3  feat: audit_log table captures domain events
- [ ] 3.4  tec: outbox adapter publishes domain events to audit_log

### Phase 4 — admin auth hardening
- [ ] 4.1  feat: admin password verified against argon2id hash
- [ ] 4.2  feat: admin login uses constant-time compare
- [ ] 4.3  feat: per-IP sliding-window lockout after 5 failed logins
- [ ] 4.4  tec: session cookie honours COOKIE_SECURE in prod
- [ ] 4.5  test: cover argon2 verify and the lockout ledger

### Phase 5 — operational readiness
- [ ] 5.1  feat: access log middleware emits structured JSON lines
- [ ] 5.2  feat: every request carries an X-Request-Id propagated into logs
- [ ] 5.3  feat: /healthz and /readyz expose liveness and readiness
- [ ] 5.4  feat: graceful shutdown drains in-flight requests on SIGTERM
- [ ] 5.5  tec: pg pool gains acquire/idle timeouts and test-before-acquire
- [ ] 5.6  tec: container runs as non-root with a pinned base image and a HEALTHCHECK
- [ ] 5.7  tec: CI enforces fmt --check, clippy -D warnings, cargo audit, docker build

### Phase 6 — frontend design system
- [ ] 6.1  tec: introduce semantic design tokens in tailwind.config.js
- [ ] 6.2  tec: extract Button, Input, Select, Surface, Stack, Badge primitives
- [ ] 6.3  fix: define the missing slide-in keyframes referenced by the toast
- [ ] 6.4  tec: collapse the three CRUD pages onto a shared CrudPage scaffold
- [ ] 6.5  fix: modal traps focus, closes on ESC, and restores focus on close
- [ ] 6.6  fix: clipboard copy uses the clipboard API directly (no eval)
- [ ] 6.7  fix: charts are invoked via a typed wasm_bindgen extern
- [ ] 6.8  fix: keys-page search debounces with a single cancellable timer
- [ ] 6.9  fix: key form surfaces submit errors through the toast
- [ ] 6.10 feat: keys page filters round-trip through URL query params
- [ ] 6.11 tec: move components/ and pages/ under ui/ feature folders

### Phase 7 — polish
- [ ] 7.1  docs: rewrite README and CLAUDE.md against the new layout
- [ ] 7.2  docs: ARCHITECTURE.md documents the bounded contexts and flow
- [ ] 7.3  feat: /metrics exposes auth decisions, cache hits, denial reasons (optional)

## Log

(Each commit appends one line here: `<id> <short-sha> <date> — <note>`)

## Blockers

(None yet.)

## How to resume

1. Read this file — the first unchecked box is the next commit.
2. Read `/root/.claude/plans/use-razif-coding-style-audit-current-velvet-lampson.md` for the full plan context.
3. Implement the slice for that commit.
4. Run `make check` — never commit red.
5. Commit code + PROGRESS.md update together with the plan's commit message.
6. Tick the box, append a Log line.
7. At phase boundaries, also run `make e2e` before starting the next phase.
8. Loop.

Stop if a commit fails `make check`/`make e2e` in a way that needs a product decision — record the reason in Blockers and stop.
