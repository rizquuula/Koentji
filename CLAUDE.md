# Koentji — Claude Code Guide

Short, code-adjacent guide for coding agents. For the deep dive, see [ARCHITECTURE.md](ARCHITECTURE.md).

## What this project is

One Rust binary serving two surfaces:

1. **`POST /v1/auth`** — public API. External apps send `{ auth_key, auth_device, rate_limit_usage }` and get back a `{ subscription, rate_limit_* }` envelope (200/401/429). Envelope is stable and bilingual (en/id error messages).
2. **Admin dashboard** — Leptos SSR + WASM hydration. Admin issues / revokes / reassigns keys, manages subscription tiers and rate-limit intervals.

Both share one Postgres + one Moka auth cache.

## Architectural stance

**Domain-driven design with strict layering.** Dependencies point inward:

```
interface/ ──► application/ ──► domain/ ◄── infrastructure/
 (Actix,        (use cases,      (entities,    (Postgres,
  Leptos)        orchestration)   value objs,    Moka,
                                  ports, events) argon2)
```

- `domain/` has no framework imports — no `sqlx`, no `actix`, no `leptos`.
- `infrastructure/` implements the ports defined in `domain/`.
- `application/` orchestrates; never holds business rules.
- `interface/http/` and `src/server/` are thin adapters that parse inputs → invoke a use case → render a response.

Don't let `sqlx::FromRow` or `actix_web::HttpRequest` leak into `domain/`. If you're tempted, add an adapter.

### Bounded contexts

| Context            | Module                        | Purpose                                                       |
|--------------------|-------------------------------|---------------------------------------------------------------|
| Authentication     | `domain/authentication`       | Hot path: `AuthKey`, `DeviceId`, `IssuedKey`, `AuthDecision`, `DenialReason`, rate-limit ledger |
| Admin access       | `domain/admin_access`         | `AdminCredentials` (argon2id), constant-time compare, per-IP login-attempt ledger |
| Key management     | `application/issue_key.rs` …  | Admin verbs: IssueKey, RevokeKey, ReassignDevice, ResetRateLimit, ExtendExpiration |

## Directory map

```
src/
├── main.rs                       Actix wiring only (use cases live in application/)
├── app.rs                        Leptos router
├── auth.rs                       Admin session server fns (login/logout)
├── db.rs                         Pool + migration runner
├── rate_limit.rs                 The atomic consume SQL helper
├── error.rs                      Shared error types
│
├── domain/
│   ├── authentication/
│   │   ├── auth_key.rs           Value object, validated
│   │   ├── device_id.rs          VO + the `-` unclaimed sentinel
│   │   ├── rate_limit.rs         Amount/Usage/Window/RemainingLedger VOs
│   │   ├── subscription_name.rs
│   │   ├── auth_decision.rs      Allowed(snapshot) | Denied(reason)
│   │   ├── issued_key.rs         Aggregate: authorize(), revoke(), reassign_to(), reset_rate_limit(), extend_until()
│   │   ├── issued_key_repository.rs  Port
│   │   ├── auth_cache_port.rs    Port
│   │   ├── audit_event_port.rs   Port
│   │   └── events.rs             Past-tense domain events
│   ├── admin_access/
│   │   ├── admin_credentials.rs  argon2id PHC + plaintext-fallback flavours
│   │   ├── constant_time.rs
│   │   └── login_attempt_ledger.rs   In-memory sliding window
│   └── errors.rs
│
├── application/                  One file per use case
│   ├── authenticate_api_key.rs   The /v1/auth use case
│   ├── issue_key.rs / revoke_key.rs / reassign_device.rs
│   ├── reset_rate_limit.rs / extend_expiration.rs
│
├── infrastructure/
│   ├── postgres/
│   │   ├── issued_key_repository.rs   Implements the domain port
│   │   └── audit_event_repository.rs  Writes domain events to `audit_log`
│   ├── cache/moka_auth_cache.rs       Implements AuthCachePort
│   ├── hashing/argon2_hasher.rs
│   └── telemetry/                     Request-id middleware, access log
│
├── interface/http/
│   ├── auth_endpoint.rs          Thin /v1/auth adapter
│   ├── i18n.rs                   DenialReason → {en, id} mapping
│   └── health.rs                 /healthz (liveness) + /readyz (pool ping)
│
├── server/                       Leptos server functions (admin CRUD)
│   └── key_service.rs / subscription_service.rs / rate_limit_service.rs / stats_service.rs
│
├── ui/                           Leptos components by feature folder
│   ├── design/                   Tokens + primitives (Button, Input, Modal, Select, DataTable, …)
│   ├── shell/                    Layout + nav
│   └── keys/ subscriptions/ rate_limits/ dashboard/ admin_access/ marketing/
│
└── bin/hash_admin_password.rs    CLI helper for ADMIN_PASSWORD_HASH
```

## Hot-path flow (`POST /v1/auth`)

1. `interface/http/auth_endpoint.rs` parses into `AuthKey` + `DeviceId`.
2. `application::AuthenticateApiKey` checks the `AuthCachePort`; on miss it calls `IssuedKeyRepository::find`.
3. If the row is missing and the `auth_key == FREE_TRIAL_KEY`, it tries `claim_free_trial` (inserts/rebinds a row with expiry on the 1st of next month UTC).
4. `IssuedKey::authorize(usage, now)` returns `AuthDecision::Allowed(snapshot) | Denied(reason)` — **pure domain logic, testable without a DB.**
5. On `Allowed`, `IssuedKeyRepository::consume_quota` runs a single atomic `UPDATE … RETURNING` that decides reset-vs-decrement in SQL (no read-modify-write race).
6. `DenialEnvelope::from_reason` renders a byte-identical `{ error: { en, id }, message }` envelope.

**The envelope is stable.** Any change there is a breaking change and needs a `/v2/auth`.

## Admin verb flow

Use cases live in `application/` and compose three ports: `IssuedKeyRepository` + `AuthCachePort` + `AuditEventPort`. Every state-mutating verb evicts the cache on success and publishes a past-tense domain event (`KeyIssued`, `KeyRevoked`, `DeviceReassigned`, `RateLimitReset`, `KeyExpirationExtended`) to `audit_log` via the fire-and-forget Postgres adapter. Cache misses and unknown-id paths are explicit — no silent fallthrough.

`ReassignDevice` is the only verb that evicts **two** cache keys (`(key, prev_dev)` and `(key, new_dev)`) — previous device could still be serving authenticated traffic from a replica.

## Key design decisions

- **Envelope stability over envelope purity.** The `/v1/auth` JSON shape is frozen — `interface/http/i18n.rs` renders legacy en/id strings byte-for-byte.
- **Atomic rate-limit consume.** One `UPDATE … SET remaining = GREATEST(remaining - $usage, 0) WHERE remaining > $usage RETURNING …`. If the `RETURNING` is empty, it's a 429.
- **Free-trial marker**, not a dedicated endpoint. Sending `auth_key == FREE_TRIAL_KEY` on an unknown `(key, device)` pair auto-provisions a row expiring on the 1st of next month UTC.
- **Unclaimed device sentinel.** `device_id = '-'` means "pre-issued, not yet bound." First call with that key adopts the sentinel row for the caller's real device.
- **Single-admin, in-memory lockout.** `LoginAttemptLedger` is a per-process sliding window (5 failures / 5 min) — fine for single-replica. Multi-replica would need Redis or equivalent.
- **In-process auth cache.** `MokaAuthCache` holds the full `IssuedKey` aggregate under `(AuthKey, DeviceId)`. TTL is `AUTH_CACHE_TTL_SECONDS`.

## Database

Postgres. Migrations in `migrations/` run automatically at server start (or via `make migrate`).

| Table                  | Rows                                                      |
|------------------------|-----------------------------------------------------------|
| `authentication_keys`  | keys + device binding + rate-limit ledger + lifecycle timestamps |
| `subscription_types`   | plan catalog + their default rate-limit amount + interval |
| `rate_limit_intervals` | named windows (hourly/daily/monthly/…)                    |
| `audit_log`            | domain events as JSONB (append-only)                      |

Schema invariants enforced in SQL: `UNIQUE(key, device_id)` on `authentication_keys`, which is also the hot-path composite index.

## Testing

Three layers:

- **Pure domain tests** (e.g. `IssuedKey::authorize`) — no DB, no async runtime. Cover denial reasons, clock boundaries, ledger arithmetic.
- **Postgres integration tests** (`tests/postgres_*.rs`) — hit a real DB via `sqlx`. Serialised through `--test-threads=1`.
- **Playwright e2e** (`end2end/`) — drives the live server; pins both admin flows and the `/v1/auth` envelope (77 tests across chromium + webkit-smoke + api).

`make check` runs `fmt --check + clippy -D warnings + test`. `make e2e` runs Playwright. Never commit red.

## Environment variables

See [.env.example](.env.example). Load order for the admin password: `ADMIN_PASSWORD_HASH` (argon2id PHC) wins; `ADMIN_PASSWORD` (plaintext) is a dev-only fallback and logs a warning at boot.

## Make targets

```
make dev                    # cargo leptos watch + tailwind
make migrate                # apply pending SQL migrations
make check                  # fmt-check + clippy -D warnings + cargo test (the safety gate)
make e2e                    # Playwright
make hash-admin-password    # argon2id hash for ADMIN_PASSWORD_HASH
make docker-up              # full stack
make refactor-status        # staged DDD remediation progress
```

## When editing

- **`/v1/auth` envelope**: never change fields, never rename error keys, never change status codes. If you must, open a `/v2/auth`.
- **Domain layer**: keep it framework-free. Add a port before you add an import.
- **`make check` must stay green**, including `clippy -D warnings`. If a lint is wrong for the context, add a scoped `#[allow]` with a one-line reason — don't relax the Makefile.
- **Commits**: one feature per commit; conventional prefix (`feat|fix|tec|test|docs|chore`); domain vocabulary in the message.
- **Tests should speak the a11y/role contract**, not Tailwind classes. `page.getByRole('alertdialog', { name: '…' }).getByRole('button', { name: … })` beats `page.locator('button.bg-red-600')`.
