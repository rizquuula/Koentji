# Koentji

API key management dashboard and authentication service. Issues, manages, and revokes API keys with subscription tiers and configurable rate limits. External applications authenticate their users against a single public endpoint: `POST /v1/auth`.

One Rust binary serves the admin dashboard (Leptos SSR + WASM hydration) and the public `/v1/auth` API from the same Actix-Web process.

## Stack

- **Backend**: Rust · Actix-Web · SQLx
- **Frontend**: [Leptos](https://leptos.dev) (SSR + WASM) · TailwindCSS
- **Database**: PostgreSQL (migrations embedded via `build.rs`)
- **Analytics store**: ClickHouse (migrations embedded via `build.rs`)
- **Auth cache**: Moka in-memory LRU behind a domain port

## Quick start

### With Docker (recommended)

```bash
cp .env.example .env          # adjust values as needed
make docker-up                # starts app + db
```

### Local development

```bash
cp .env.example .env
make docker-up-db             # just the DB
make migrate                  # run pending SQL migrations
make dev                      # cargo leptos watch + tailwind
```

Open <http://localhost:3000>.

## Public API

### `POST /v1/auth`

```json
{
  "auth_key": "YOUR_KEY",
  "auth_device": "DEVICE_ID",
  "rate_limit_usage": 1
}
```

`rate_limit_usage` is accepted as `f64`, so fractional consume is allowed (`1` still works — integer JSON deserialises into `1.0`). `rate_limit_remaining` in the response is an integer (ceil-shimmed from internal `f64` storage). Every request also emits a per-request analytics event to ClickHouse. Envelope is **frozen**.

| Status | Meaning                                                  |
|--------|----------------------------------------------------------|
| 200    | Key valid; envelope reports subscription + remaining quota |
| 401    | Key unknown, revoked, expired, or free trial ended        |
| 429    | Rate limit exceeded                                      |

Interactive OpenAPI docs at <http://localhost:3000/docs/> when the server is running.

## Environment

See [.env.example](.env.example). Required in every environment:

| Variable                | Default         | Purpose                                          |
|-------------------------|-----------------|--------------------------------------------------|
| `DATABASE_URL`          | —               | Postgres DSN                                     |
| `SECRET_KEY`            | —               | Cookie signing key (≥ 64 bytes)                  |
| `ADMIN_USERNAME`        | `admin`         | Dashboard login                                  |
| `ADMIN_PASSWORD_HASH`   | —               | **Use in production.** argon2id PHC string; generate via `make hash-admin-password PASSWORD=...` |
| `ADMIN_PASSWORD`        | `admin`         | Plaintext fallback — dev/e2e only; logs a warning at boot |
| `FREE_TRIAL_KEY`        | `FREE_TRIAL`    | Marker value that auto-provisions a trial row on first call |
| `AUTH_CACHE_TTL_SECONDS`| `900`           | Moka auth-cache TTL                              |
| `COOKIE_SECURE`         | `true`          | Set to `false` only for plain-HTTP local dev     |
| `WORKERS`               | `4`             | Actix worker threads                             |
| `CLICKHOUSE_URL`        | —               | ClickHouse HTTP DSN (e.g. `http://clickhouse:8123/default`) |
| `CLICKHOUSE_PASSWORD`   | —               | ClickHouse password                              |

## Makefile

Every non-trivial command has a target. Run `make` (or `make help`) for the full list. Daily drivers:

| Target                    | What it does                                                                |
|---------------------------|-----------------------------------------------------------------------------|
| `make dev`                | dev server with live reload                                                 |
| `make migrate`            | apply pending SQL migrations                                                |
| `make check`              | `fmt --check` + `clippy -D warnings` + `cargo test` — the safety gate       |
| `make e2e`                | Playwright suite (admin CRUD + `/v1/auth` contract + hydration smoke)       |
| `make hash-admin-password`| print an argon2id hash for `ADMIN_PASSWORD_HASH`                            |
| `make docker-up`          | start the full stack                                                        |
| `make refactor-status`    | show the staged refactor checklist                                          |

## Repository layout

```
src/
├── main.rs              Actix server wiring (use cases live in application/)
├── app.rs               Leptos router
├── auth.rs              Admin login/logout server fns
├── db.rs                Pool + migration runner
├── domain/              Entities, value objects, ports, events — no framework deps
├── application/         Use cases (Authenticate, IssueKey, RevokeKey, …)
├── infrastructure/      Postgres repositories, Moka cache, argon2, telemetry
├── interface/http/      Actix adapters: /v1/auth endpoint, i18n, /healthz, /readyz
├── server/              Leptos server functions (thin adapters to application/)
└── ui/                  Leptos components organised by feature folder
    ├── design/          Tokens + primitives (Button, Input, Modal, DataTable, …)
    ├── shell/           Layout + nav
    ├── keys/ subscriptions/ rate_limits/ dashboard/ admin_access/ analytics/ marketing/
migrations/              SQL migrations (embedded into the binary)
clickhouse/migrations/   ClickHouse migrations (embedded into the binary)
tests/                   Rust integration tests (Postgres-backed harness)
end2end/                 Playwright e2e suite
```

## Testing

```bash
make check   # fmt + clippy + unit/integration tests
make e2e     # Playwright suite against a live server
```

Integration tests share one Postgres DB and coordinate via a single `reset()` helper — the `Makefile` wraps them in `--test-threads=1` so the truncate doesn't race.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the bounded contexts, the `/v1/auth` hot-path flow, and the admin command model. [CLAUDE.md](CLAUDE.md) is the short, code-adjacent guide aimed at coding agents working in this repo.
