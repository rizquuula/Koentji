# Koentji Refactor — Progress

- Plan: `/root/.claude/plans/use-razif-coding-style-audit-current-velvet-lampson.md`
- Started: 2026-04-17
- Current phase: 2
- Next commit: 2.2

## Checklist

### Phase 0 — safety net
- [x] 0.1  test: add integration harness and domain-test helpers
- [x] 0.2  fix: bind custom date-range parameters in dashboard stats query
- [x] 0.3  fix: decrement rate limit atomically on /v1/auth
- [x] 0.4  chore: drop stale agAuth/ references from docs
- [x] 0.5  tec: make check aggregates fmt + clippy + test; CI runs it

### Phase 1 — domain extraction
- [x] 1.1  tec: introduce domain module skeleton and value objects
- [x] 1.2  tec: extract IssuedKey aggregate with lifecycle verbs
- [x] 1.3  tec: define AuthDenialReason enum with en/id mapping at HTTP edge
- [x] 1.4  tec: introduce IssuedKeyRepository port + Postgres adapter
- [x] 1.5  tec: hide Moka auth cache behind AuthCachePort
- [x] 1.6  feat: route /v1/auth through AuthenticateApiKey use case
- [x] 1.7  test: cover IssuedKey.authorize across all denial reasons
- [x] 1.8  test: integration tests for Postgres IssuedKeyRepository

### Phase 2 — admin verbs
- [x] 2.1  feat: issuing a key emits KeyIssued and returns an IssuedKey
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

- 0.1  2026-04-17 — `tests/common/{mod,clock,db,key_builder}.rs` + `tests/harness_smoke.rs`; shared test DB, KeyBuilder, TestClock. Pre-existing clippy errors in `src/` are unrelated and will be addressed later.
- 0.2  2026-04-17 — `src/server/stats_service.rs` rewritten to bind `Option<DateTime<Utc>>` into every query. `custom` range now parses strictly as YYYY-MM-DD — malformed input degrades to (None, None). 6 regression tests in `tests/stats_date_range.rs`.
- 0.3  2026-04-17 — new `src/rate_limit.rs` with `consume_rate_limit`: a single `UPDATE … RETURNING` decides reset-vs-decrement in-SQL and locks the row, closing the read-modify-write leak. `/v1/auth` no longer spawns a fire-and-forget writer. 6 regression tests in `tests/rate_limit_atomic.rs` including a 10-concurrent-spawn race probe. `tests/common/db.rs` hands each `#[tokio::test]` a runtime-local pool (PgPool isn't runtime-portable) while keeping DB setup + migrations one-shot per process.
- 0.4  2026-04-17 — `README.md` + `CLAUDE.md` no longer claim a sibling `agAuth/` crate; replaced with an accurate single-crate layer table that mentions the `tests/` + `end2end/` suites.
- 0.5  2026-04-17 — `make test` no longer calls `cargo fmt` (which rewrites in CI); new `make check = fmt-check + clippy (-D warnings, --tests) + test`. Also cleared the ~10 pre-existing `clone_on_copy` / `if let`-rewriteable clippy warnings in `src/pages` / `src/components` / `src/main.rs` so the gate is actually usable. `.github/workflows/test.yml` now runs `make check` against a Postgres service container.
- P0-e2e 2026-04-17 — `make e2e` (api project) run at Phase 0/1 boundary. Found the legacy off-by-one: old handler returned 429 as soon as post-decrement remaining hit `<= 0`, so only `daily - 1` consumes per window are actually usable. Preserved the semantic in `rate_limit.rs` (`remaining > usage`, `daily > usage`) and realigned the unit tests. All 10 api e2e tests green on chromium.
- 1.1  2026-04-17 — new `src/domain/` module with the `authentication` bounded context and `DomainError`. Value objects: `AuthKey`, `DeviceId` (including the `"-"` unclaimed sentinel), `RateLimitAmount`, `RateLimitUsage`, `RateLimitWindow`, `SubscriptionName`. Every one has `parse/new` validation and unit tests covering empty / over-long / negative / zero branches — 22 tests. Nothing is wired into the HTTP path yet; this is the vocabulary that 1.2–1.6 will lean on.
- 1.2  2026-04-17 — `src/domain/authentication/auth_decision.rs` introduces `AuthDecision` + `DenialReason`; `src/domain/authentication/issued_key.rs` hosts the `IssuedKey` aggregate with `authorize(usage, now) -> AuthDecision` (pure) plus the lifecycle verbs `revoke`, `reassign_to`, `reset_rate_limit`, `extend_until`. Authorize preserves the legacy off-by-one (`daily > usage`, `remaining > usage`) and the free-trial-vs-admin expiry split. 12 aggregate tests cover active/revoked/idempotent-revoke/expired-admin/expired-trial/off-by-one/usage>=daily/window-reset/null-updated/reassign-preserves-ledger/reset-restores-daily/extend-set-clear.
- 1.3  2026-04-17 — new `src/interface/http/i18n.rs` with `DenialEnvelope::from_reason` + `status_code(reason)`. Strings copied byte-for-byte from the inline `json!` blocks in `src/main.rs` so e2e text assertions keep passing. 5 unit tests pin each denial shape + its legacy status code (Unknown/Revoked/Expired/FreeTrial → 401, RateLimit → 429). Not wired yet — 1.6 will mount the new envelope via the `AuthenticateApiKey` use case.
- 1.4  2026-04-17 — new `src/domain/authentication/issued_key_repository.rs` defines the port (`find` + `consume_quota` via `async_trait`, `ConsumeOutcome`, `RepositoryError`). New `src/infrastructure/postgres/issued_key_repository.rs` implements it — `find` hydrates an `IssuedKey` via a private `IssuedKeyRow` so `sqlx::FromRow` never leaks onto domain types; `consume_quota` reuses the atomic `UPDATE … RETURNING` from 0.3 verbatim. `async-trait` added to deps under the `ssr` feature. Not wired into `/v1/auth` yet — 1.6 hooks it up.
- 1.5  2026-04-17 — new `src/domain/authentication/auth_cache_port.rs` (`AuthCachePort` with `get`/`put`/`invalidate` over `(AuthKey, DeviceId)`). New `src/infrastructure/cache/moka_auth_cache.rs` adapts Moka to the port and stores the full `IssuedKey` aggregate rather than the legacy DB row. Legacy `src/cache.rs` stays until 1.6 swaps the endpoint.
- 1.6  2026-04-17 — `/v1/auth` now flows through `koentji::application::AuthenticateApiKey` (new) → `koentji::interface::http::auth_endpoint` (new). `src/main.rs` shrinks from 634 lines to ~188 (wiring only). The handler is non-generic and takes `Arc<AuthenticateApiKey>`; the use case holds `Arc<dyn IssuedKeyRepository>` + `Arc<dyn AuthCachePort>` so the actix `#[post]` macro doesn't fight type parameters. `IssuedKey` gains `username`/`email` (envelope echoes them). `claim_free_trial` ported onto the repository adapter preserving the two-branch INSERT/rebind upsert + 1st-of-next-month UTC calendar math. Envelope byte-identical: `DenialEnvelope::from_reason` + `status_code` renders the same `{ error: { en, id }, message }` as before. Legacy `src/cache.rs` and `src/server/key_service.rs` unchanged (Phase 2 migrates them). 39 unit + 3 harness + 6 rate_limit + 6 stats tests all green.
- 1.7  2026-04-17 — +24 `IssuedKey.authorize` tests covering: priority-of-denial (revoked beats expired beats rate-limit, free-trial-ended vs rate-limit), clock boundaries (==expiry denies, nanosecond before allows; window reset fires at `==` boundary, not one ns early), ledger arithmetic edge cases (remaining==usage+1 allowed, usage==daily denied, `usage==0` is allowed without decrement), non-default window (60s window resets after 61s), purity (authorize does not mutate), `updated_at` stamping, denial timestamp round-trip (revoked/expired/free-trial), and lifecycle verb interactions (reassign/reset don't unrevoke; extend_until revives an expired key; None expiry is endless). 36 authorize-focused tests in total (>30 plan target). 78 tests across 6 suites all green.
- 1.8  2026-04-17 — new `tests/postgres_issued_key_repository.rs` (12 tests) exercises the real DB adapter: `find` hydrates identity + ledger + username/email and returns revoked rows (they are not treated as missing), default window falls back to 86_400s. `consume_quota` allows within quota, refuses at the legacy off-by-one, resets a stale window, never oversells under 20 concurrent spawns (exactly 19 Allowed, final remaining=1). `claim_free_trial` inserts on marker match, rebinds a pre-issued `device_id='-'` row (final count=1), returns None for a plain unknown key. Every test first calls `fresh_pool()` → `reset` so cross-pollution is impossible. 90 rust tests across 7 suites + 77 Playwright tests all green — Phase 1 boundary verified.
- 2.1  2026-04-17 — new `src/application/issue_key.rs` (`IssueKey` use case) + `IssueKeyCommand` on the repository port + `issue_key` on the Postgres adapter. `server::key_service::create_key` now parses the request into domain value objects (`AuthKey`, `DeviceId`, `SubscriptionName`, `RateLimitAmount`) and delegates to `IssueKey`; the server-fn contract still returns `AuthenticationKey` (full DB row incl. timestamps / audit fields), so the adapter re-fetches by id after the insert — the domain aggregate intentionally doesn't carry those. `main.rs` wires `Arc<IssueKey>` alongside `AuthenticateApiKey`; both share the same `Arc<dyn IssuedKeyRepository>`. `KeyIssued` past-tense log line emitted from the use case — the formal outbox/audit adapter lands in 3.4. No new tests this commit (2.5 covers the admin verbs as a suite); envelope + existing 90 rust tests still green.

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
