# Koentji Lab — Claude Code Guide

## Project Overview

**Koentji Lab** is an API key management dashboard and authentication service. It allows an admin to issue, manage, and revoke API keys with subscription tiers and rate limits. External applications authenticate their users by calling the `/v1/auth` endpoint.

The project has two components:

| Component | Tech | Purpose |
|-----------|------|---------|
| `./` (main) | Rust · Leptos · Actix-Web · PostgreSQL | Full-stack web app (SSR + WASM hydration) + public auth API |
| `./agAuth/` | Python · FastAPI (inferred) | Standalone Python auth microservice (legacy / alternative) |

---

## Architecture

```
src/
├── main.rs            — Actix-Web server setup, /v1/auth endpoint, OpenAPI (utoipa)
├── lib.rs             — Crate root, hydrate() entry for WASM
├── app.rs             — Leptos <App/> router
├── auth.rs            — Admin session login/logout server functions
├── cache.rs           — Moka in-memory cache for auth lookups (AuthCache)
├── db.rs              — SQLx pool creation + migration runner
├── error.rs           — Shared error types
├── models.rs          — AuthenticationKey and related DB structs
├── components/        — Reusable Leptos UI components
│   ├── key_table.rs / key_row.rs / key_form.rs
│   ├── modal.rs       — Confirmation modals
│   ├── toast.rs       — Toast notifications
│   ├── stats_cards.rs / charts.rs / date_range_picker.rs
│   └── layout.rs      — Shell / nav layout
├── pages/             — Full page components
│   ├── login.rs / dashboard.rs / keys.rs
│   ├── subscriptions.rs / rate_limits.rs / quickstart.rs
└── server/            — Leptos server functions (called from frontend)
    ├── key_service.rs
    ├── subscription_service.rs
    ├── rate_limit_service.rs
    └── stats_service.rs
```

### Key design decisions

- **SSR + hydration**: Leptos renders pages server-side; the WASM bundle hydrates them in the browser.
- **Auth cache**: `AuthCache` (Moka) caches DB lookups for `AUTH_CACHE_TTL_SECONDS` (default 15 min). Cache is invalidated / updated on every successful `/v1/auth` call.
- **Rate limit reset**: Based on a configurable interval (`rate_limit_intervals` table), not a fixed daily window.
- **Free trial keys**: Sending `FREE_TRIAL_KEY` as the `auth_key` auto-creates a free-trial record for the device, expiring on the 1st of the next month.
- **Feature flags**: `ssr` feature enables all server-side deps; `hydrate` feature builds the WASM bundle.

---

## Database

PostgreSQL. Migrations live in `migrations/` and run automatically on server start (or via `make migrate`).

Key tables: `authentication_keys`, `subscription_types`, `rate_limit_intervals`.

---

## Environment Variables

See `.env.example`:

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `SECRET_KEY` | Cookie signing key (≥ 64 bytes) |
| `ADMIN_USERNAME` | Dashboard login username |
| `ADMIN_PASSWORD` | Dashboard login password |
| `FREE_TRIAL_KEY` | Magic string that triggers free-trial upsert (default: `FREE_TRIAL`) |
| `AUTH_CACHE_TTL_SECONDS` | How long auth results are cached (default: 900) |

---

## Common Commands

```bash
make dev          # Start dev server (cargo leptos watch + tailwind)
make build        # Production build
make migrate      # Run pending SQL migrations
make docker-up    # Start all containers (app + db)
make docker-up-db # Start only the DB container
make fmt          # cargo fmt
make clippy       # cargo clippy --all-features
```

---

## Public API

`POST /v1/auth` — authenticate an API key.

Swagger UI available at `/docs/` when the server is running.

Request body:
```json
{
  "auth_key": "string",
  "auth_device": "string",
  "rate_limit_usage": 1
}
```

Responses: `200 OK`, `401 Unauthorized`, `429 Too Many Requests`, `500 Internal Server Error`.
