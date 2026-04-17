# Koentji — Architecture

This document is the deep dive. For a quickstart see [README.md](README.md); for the code-adjacent agent guide see [CLAUDE.md](CLAUDE.md).

---

## 1 · The two surfaces

Koentji is one Rust binary serving two surfaces from one Actix-Web process:

| Surface                  | Consumer          | Shape                                                              |
|--------------------------|-------------------|--------------------------------------------------------------------|
| `POST /v1/auth`          | external apps     | Stable JSON envelope (200/401/429) — **byte-identical and frozen** |
| Admin dashboard          | one privileged admin | Leptos SSR + WASM hydration; session-cookie auth                |

They share one Postgres schema, one in-process Moka cache, and the same domain model. Nothing on the admin surface is allowed to destabilise the `/v1/auth` envelope.

---

## 2 · Layering

Dependencies point inward. **`domain/` knows nothing about frameworks.**

```
┌──────────────────────────────────────────────────────────────┐
│  interface/                                                  │
│   http/  (Actix handlers: /v1/auth, /healthz, /readyz)       │
│   leptos (src/app.rs, src/auth.rs, src/server/*)             │
└──────────────┬───────────────────────────────────────────────┘
               │ parse → use case → render
               ▼
┌──────────────────────────────────────────────────────────────┐
│  application/                                                │
│   AuthenticateApiKey · IssueKey · RevokeKey · ReassignDevice │
│   ResetRateLimit · ExtendExpiration                          │
└──────────────┬───────────────────────────────────────────────┘
               │ orchestrates
               ▼
┌──────────────────────────────────────────────────────────────┐
│  domain/                                                     │
│   authentication/  — AuthKey, DeviceId, IssuedKey,           │
│                      AuthDecision, DenialReason, events      │
│                      + IssuedKeyRepository port              │
│                      + AuthCachePort port                    │
│                      + AuditEventPort port                   │
│   admin_access/    — AdminCredentials, ConstantTime,         │
│                      LoginAttemptLedger                      │
└──────────────▲───────────────────────────────────────────────┘
               │ implements
┌──────────────┴───────────────────────────────────────────────┐
│  infrastructure/                                             │
│   postgres/  — IssuedKeyRepository, AuditEventRepository     │
│   cache/     — MokaAuthCache                                 │
│   hashing/   — Argon2Hasher                                  │
│   telemetry/ — RequestId middleware, AccessLog JSON emitter  │
└──────────────────────────────────────────────────────────────┘
```

### What the rule actually rules out

| You may not …                                         | Do this instead                               |
|-------------------------------------------------------|-----------------------------------------------|
| `#[derive(sqlx::FromRow)]` on a domain entity         | Private `…Row` DTO in `infrastructure/postgres/` that hydrates a domain aggregate |
| Import `actix_web` from `application/`                | Keep Actix in `interface/http/` — use cases take primitives and ports |
| Call `sqlx::query!` from `application/`               | Define a port method in `domain/`, impl in `infrastructure/` |
| Reach for a global `OnceLock<AuthCache>`              | Construct at wiring time, inject as `Arc<dyn AuthCachePort>` |

Every one of these was a real finding in the pre-refactor audit (see `.claude-refactor/` for the staged remediation log).

---

## 3 · Bounded contexts

```
┌──────────────────────────────┐   ┌──────────────────────────────┐
│ Authentication               │   │ Key Management               │
│  (hot path: /v1/auth)        │   │  (admin dashboard)           │
│                              │   │                              │
│  • AuthKey, DeviceId         │   │  • IssueKey                  │
│  • IssuedKey (aggregate)     │◄──│  • RevokeKey                 │
│  • AuthDecision              │   │  • ReassignDevice            │
│  • DenialReason              │   │  • ResetRateLimit            │
│  • RateLimitAmount/Usage/…   │   │  • ExtendExpiration          │
└──────────────┬───────────────┘   └──────────────┬───────────────┘
               │                                  │
               ▼                                  ▼
         ┌─────────────────────────────────────────────┐
         │  Shared SQL substrate                       │
         │  authentication_keys · audit_log            │
         └─────────────────────────────────────────────┘

┌──────────────────────────────┐
│ Admin Access                 │
│  • AdminCredentials          │
│    (argon2id PHC ‖ plaintext)│
│  • ConstantTime compare      │
│  • LoginAttemptLedger        │
│    (in-process sliding window)│
└──────────────────────────────┘
```

**Why three contexts, not one.** Authentication is a hot, stateless, pure-ish decision system with stability guarantees that dominate every other concern. Key management is a state-mutating admin interface that has to cooperate with Authentication's cache but shouldn't dictate its shape. Admin Access is a self-contained gate into the dashboard with its own lifecycle (lockout, hashing). Folding any two into one makes every commit in one context risk destabilising the other.

---

## 4 · Hot path — `POST /v1/auth`

```
     ┌───────────────────────────┐
     │ Client                    │
     └──────────────┬────────────┘
                    │ JSON: { auth_key, auth_device, rate_limit_usage }
                    ▼
┌──────────────────────────────────────────────────────────────┐
│ interface/http/auth_endpoint.rs    (thin Actix adapter)      │
│  • Parse AuthKey, DeviceId                                   │
│  • Default rate_limit_usage = 1                              │
│  • Invoke AuthenticateApiKey                                 │
└──────────────┬───────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────────┐
│ application/authenticate_api_key.rs                          │
│                                                              │
│  1. cache.get(key, device) ────────► HIT: skip to step 3     │
│  2. repo.find(key, device) ───► miss + key==FREE_TRIAL_KEY   │
│           │                     ──► repo.claim_free_trial()  │
│           │                          (upsert; expiry = 1st   │
│           │                          of next month UTC)      │
│           ▼                                                  │
│  3. IssuedKey.authorize(usage, now) ─► AuthDecision          │
│                                        │ Allowed(snapshot)   │
│                                        │ Denied(reason)      │
│                                        ▼                     │
│  4. On Allowed: repo.consume_quota()  (atomic UPDATE …       │
│                                        RETURNING — see §5)   │
│     On Denied:  stop                                         │
│  5. cache.put(key, device, snapshot)                         │
└──────────────┬───────────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────────────┐
│ interface/http/i18n.rs                                       │
│  DenialEnvelope::from_reason(reason)                         │
│   → byte-identical { error: { en, id }, message }            │
│  status_code(reason) → 401 | 429                             │
└──────────────────────────────────────────────────────────────┘
```

`IssuedKey::authorize` is pure — no DB, no clock singleton (the caller passes `now`), no async. That's where the denial-priority rules live: revoked beats expired beats rate-limit, free-trial-expired beats rate-limit, rate-limit beats allowed. It has 36 unit tests covering clock boundaries and ledger arithmetic.

---

## 5 · Atomic rate-limit consume (the race we closed)

The old handler read the row, subtracted, and scheduled a fire-and-forget write. Under concurrent requests the ledger leaked: two simultaneous reads both saw the same `remaining`, both decided "allowed," both wrote back the same new value. A client could over-run its quota by `concurrency − 1`.

The fix is one SQL statement that atomically computes reset-vs-decrement and locks the row for the duration:

```sql
UPDATE authentication_keys
SET remaining = CASE
      WHEN window_elapsed THEN daily - $usage
      ELSE remaining - $usage
    END,
    updated_at = $now
WHERE key = $k AND device_id = $d
  AND (window_elapsed OR remaining >= $usage)
  AND daily >= $usage
RETURNING remaining, updated_at
```

If the `RETURNING` comes back empty, it's a 429 — no retry, no race. `N+1` concurrent `spawn`s at a key with `daily=N` produce exactly `N` Allowed outcomes and one refusal. See `tests/postgres_issued_key_repository.rs` for the concurrency probe.

**Predicate semantic.** A request with `usage == remaining` is Allowed and drops remaining to exactly `0`; the next request is refused. `daily` is fully consumable per window. (Prior revisions kept a `>` predicate — the "legacy off-by-one" — which left the last slot unreachable. That was corrected so admin-UI `remaining/daily` matches operator intuition; the `/v1/auth` envelope fields and status codes are unchanged.)

---

## 6 · Admin verb flow

```
┌───────────────────────────────┐
│ Leptos server fn              │
│  (src/server/key_service.rs)  │
└──────────────┬────────────────┘
               │ parse DTO → AuthKey, DeviceId, SubscriptionName, …
               ▼
┌───────────────────────────────┐
│ application/<verb>.rs         │
│  IssueKey / RevokeKey /       │
│  ReassignDevice / …           │
└──────────────┬────────────────┘
               │ orchestrates these three ports:
               ▼
      ┌───────────────────────┐
      │ IssuedKeyRepository   │  ← Postgres; atomic verb SQL
      │ AuthCachePort         │  ← Moka; evict on success
      │ AuditEventPort        │  ← Postgres `audit_log`; fire-and-forget
      └───────────────────────┘
```

**Invariants.**

- **Every state-mutating verb that succeeds evicts the auth cache.** Otherwise a revoked key keeps working until its TTL expires.
- **`ReassignDevice` evicts two entries:** `(key, prev_device)` *and* `(key, new_device)`. The previous-device entry could still be serving authenticated traffic from another worker.
- **`RevokeKey` is idempotent.** A second revoke preserves the original timestamp via `COALESCE(deleted_at, NOW())`.
- **Unknown ids don't touch the cache.** Verbs short-circuit so nobody pays for a useless invalidation.
- **Audit writes are fire-and-forget.** A failed `audit_log` insert logs `warn!` but never bubbles back — audit must not fail the witnessed operation. The domain event is still emitted to `log!` for structured-log correlation.

---

## 7 · Admin access

Single-admin model, hardened:

```
Login request
      │
      ▼
┌─────────────────────────────┐     LockoutPolicy { max = 5, window = 5 min }
│ LoginAttemptLedger::check() │───► LockedOut → 429 + Retry-After
└──────────────┬──────────────┘
               │ Allowed
               ▼
┌──────────────────────────────────────────────────────────┐
│ AdminCredentials (loaded from env, precedence below)     │
│   1. ADMIN_PASSWORD_HASH — argon2id PHC (production)     │
│   2. ADMIN_PASSWORD      — plaintext (dev/e2e only)      │
│                                                          │
│ Always-runs verification (bitwise-AND of both checks)    │
│   user_ok = constant_time_eq(username, env_username)     │
│   pw_ok   = creds.verify(password)                       │
│   if user_ok & pw_ok { … }                               │
│                                                          │
│ Wrong-user and wrong-password are now timing-indistinguishable. │
└──────────────────────────────────────────────────────────┘
```

`LoginAttemptLedger` is a `Mutex<HashMap<IpString, VecDeque<DateTime<Utc>>>>`. Prune semantics: a failure whose age equals the window is aged out (open interval at the far edge), so an attacker who waits exactly `window` always sees a fresh slot. On success the IP clears.

Session cookie `Secure` attribute is driven by `COOKIE_SECURE` (default `true`; e2e sets `false` so Playwright's plain-HTTP driver keeps its session).

---

## 8 · Data model

### `authentication_keys`

The hot-path row. Columns: `id`, `key`, `device_id`, `username`, `email`, `subscription_type_id`, `rate_limit_daily`, `rate_limit_remaining`, `rate_limit_interval_id`, `rate_limit_window_start`, `expired_at`, `deleted_at`, `created_at`, `updated_at`.

- `UNIQUE(key, device_id)` — migration 004. This is also the hot-path index (Postgres treats a unique btree and a plain btree identically for equality lookups, so a separate composite index is duplication).
- `device_id = '-'` is the **unclaimed sentinel**: a key pre-issued but not yet bound to a device. First `/v1/auth` call with that key adopts the sentinel row for the caller's real device.

### `subscription_types`

Plan catalog: `id`, `name`, `display_name`, `rate_limit_amount`, `rate_limit_interval_id`, `is_active`.

### `rate_limit_intervals`

Named windows: `id`, `name`, `display_name`, `duration_seconds`, `is_active`. Rate-limit reset is driven by these intervals, not a fixed daily clock.

### `audit_log`

Append-only stream of domain events. Columns: `id`, `event_type` (`KeyIssued`, `KeyRevoked`, `DeviceReassigned`, `RateLimitReset`, `KeyExpirationExtended`), `aggregate_id`, `actor`, `payload JSONB`, `occurred_at`.

Indexes: `idx_audit_log_occurred_at` (recent-first scrolling) and a partial `idx_audit_log_aggregate` on `(aggregate_id, occurred_at DESC) WHERE aggregate_id IS NOT NULL` (per-key history).

JSONB payload so the schema can evolve without another migration.

---

## 9 · Frontend architecture

The admin dashboard is Leptos SSR + WASM hydration. Feature-folder layout:

```
src/ui/
├── design/      tokens + primitives (Button, Input, Select, DataTable, Modal, …)
├── shell/       Layout + nav
├── keys/        page + KeyForm + KeyRow + KeyTable
├── subscriptions/  page + form
├── rate_limits/    page + form
├── dashboard/      page + charts + date range picker + stats cards
├── admin_access/   login page
└── marketing/      landing/about/terms/privacy/quickstart
```

### Design system

Semantic Tailwind tokens in `tailwind.config.js`:

- `colors.brand.*` — primary action blue ladder
- `colors.surface.*` — neutral backgrounds + borders
- `colors.ink.*` — text tiers (heading / body / muted / disabled)
- `colors.feedback.*` — danger / warning / success + `-ink` variants
- `borderRadius.control` / `card`
- `boxShadow.raised` / `overlay`
- `transitionDuration.quick` / `settled`
- `animate-slide-in` (for toast)

**No magic Tailwind values in component files.** If a component needs a new colour, add a token.

### Primitives

`Button` (Primary/Secondary/Danger/Ghost × width), `Input` (text-like, binds an `RwSignal<String>`, reactive `readonly`), `Select` (binds + caller composes `<option>`), `Surface` (Raised/Overlay × padding), `Stack` (Tight/Normal/Loose vertical gap with every literal class spelled out so Tailwind's JIT picks them up), `Badge` (Neutral/Brand/Success/Warning/Danger), `PageHeader`, `DataTable` (header `<th>` row gets `scope="col"` automatically), `Modal` + `ConfirmModal` (focus trap, ESC, `role="dialog"` / `role="alertdialog"`, focus restore).

### State

- **URL query params** for filters that should be shareable/back-button-safe (`KeysPage` reflects `search`, `page`, `subscription`, `status` via `leptos_router::query_signal`). `None` means "default."
- **Resource** (Leptos async signal) for server-fetched data.
- **Debounced writes** (`StoredValue<Option<TimeoutHandle>>`) cancel the pending timer before re-scheduling, so five keystrokes collapse to one fetch.

### Accessibility

- `role="dialog"` / `role="alertdialog"` on modals, `aria-modal="true"`, `aria-label` from the title, focus trap for Tab + Shift+Tab, ESC closes and restores focus to the element that opened the dialog.
- `<label for="…">` → `<input id="…">` on every form field.
- `<th scope="col">` on every table header.
- Icon-only buttons carry `aria-label`; their inner `<svg>` is `aria-hidden="true"`.
- Toast container is `role="status" aria-live="polite"`; error toasts override to `role="alert"`.

---

## 10 · Observability

| Surface              | How                                                                                  |
|----------------------|--------------------------------------------------------------------------------------|
| Access log           | `infrastructure/telemetry/access_log.rs` — one single-line JSON per request, emitted on target `http_access` (ts, method, path, status, bytes, duration_ms, referer, user_agent, request_id) |
| Request correlation  | `RequestIdMiddleware` accepts a trusted inbound `X-Request-Id` or mints a UUID v7 (time-sortable). Printable-ASCII / ≤128 bytes, else rejected (log-injection guard). Id surfaces in the access log and mirrors back in the response header. |
| Domain events        | `AuditEventPort` publishes past-tense events to `audit_log` as JSONB                 |
| Health probes        | `/healthz` liveness (no DB touch — must not fail on a transient outage); `/readyz` readiness (runs `SELECT 1` with a 2s timeout against the pool) |
| Graceful shutdown    | SIGTERM traps, Actix drains in-flight, `PgPool::close` runs afterwards               |

No Prometheus yet (7.3 was marked optional and deferred — see §12).

---

## 11 · Testing strategy

Three layers, each with a job the others can't do.

```
┌──────────────────────────────────────────────────────────────┐
│ Pure domain tests                                            │
│   IssuedKey::authorize ledger/clock/denial-priority tests    │
│   AdminCredentials / ConstantTime / LoginAttemptLedger       │
│   Value object validation                                    │
│                                                              │
│   No DB, no async runtime. Fastest feedback.                 │
└──────────────────────────────────────────────────────────────┘
                         ▼  invariants → adapters hold them
┌──────────────────────────────────────────────────────────────┐
│ Postgres integration tests (tests/postgres_*.rs)             │
│   Issued/Revoke/Reassign/Reset/Extend verb SQL               │
│   Atomic consume_quota under 20 concurrent spawns            │
│   Audit-event round-trip + closed-pool fire-and-forget       │
│   UNIQUE(key, device_id) rejection                           │
│                                                              │
│   Serialised: `cargo test -- --test-threads=1` (shared DB +  │
│   one `reset()` helper).                                     │
└──────────────────────────────────────────────────────────────┘
                         ▼  + wiring + UI
┌──────────────────────────────────────────────────────────────┐
│ Playwright end-to-end (end2end/)                             │
│   chromium: admin CRUD flows                                 │
│   api:      /v1/auth success + error + free-trial + reset    │
│   chromium-guest: login/logout/guest redirects               │
│   webkit-smoke: hydration + public pages                     │
│                                                              │
│   77 tests. Pins the envelope and the admin flows.           │
└──────────────────────────────────────────────────────────────┘
```

**What each layer protects.** Pure domain tests pin logic invariants — they don't break when the DB changes shape. Postgres integration tests pin the contract between a domain port and its SQL — they're the only layer that catches "my UPDATE should lock the row but doesn't." Playwright pins the *wire* — envelope stability, hydration, admin flow ergonomics. Removing any one layer leaves a gap that the other two can't close.

**Test naming in domain vocabulary.** `authorizes_an_active_under_quota_key`, `revoking_an_already_revoked_key_preserves_the_original_timestamp` — not `test_happy_path`.

**Tests speak the a11y/role contract, not Tailwind.** `page.getByRole('alertdialog', { name: '…' }).getByRole('button', { name: … })` stays green across token renames.

---

## 12 · Out of scope / deferred

| Ref  | Deferred                                                                               | Why                                                                 |
|------|----------------------------------------------------------------------------------------|---------------------------------------------------------------------|
| 7.3  | `/metrics` Prometheus endpoint                                                          | No consuming stack yet. Add when a second caller appears.           |
| N1   | Multi-user admin RBAC                                                                   | Single-admin is hardened; multi-user needs a table + role model.    |
| N2   | Breaking changes to the `/v1/auth` envelope (typed `reason_code`, etc.)                | Would require `/v2/auth` — different project.                       |
| N3   | Migration off Leptos SSR/WASM                                                           | —                                                                   |
| N4   | Distributed auth cache / distributed login-attempt ledger / shared audit pipeline       | Needed for multi-replica; not in scope for single-replica.          |
| N5   | FE⇄BE contract-test server                                                              | Unnecessary for a single-repo single-author surface.                |
| N6   | External-dependency mock surroundings server                                            | Koentji has no external dependencies to mock.                       |
| N7   | Full admin-dashboard i18n                                                               | `/v1/auth` is bilingual; admin surface is not a current priority.   |

---

## 13 · How this doc is meant to be used

- **Reading code for the first time?** Start here, then jump to the file you were aiming for.
- **About to change the `/v1/auth` envelope?** Stop. Read §4–§5, then propose `/v2/auth`.
- **Adding a new admin verb?** Follow §6 — port method in `domain/`, impl in `infrastructure/postgres/`, use case in `application/`, thin Leptos server fn in `src/server/`.
- **Adding a new UI primitive or token?** §9. Don't sprinkle raw Tailwind colours in feature code.
- **Debugging a rate-limit race or quota leak?** §5.
