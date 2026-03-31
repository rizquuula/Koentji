# Koentji

API key management dashboard and authentication service. Lets you issue, manage, and revoke API keys with subscription tiers and configurable rate limits. External applications authenticate their users against the `/v1/auth` endpoint.

## Stack

- **Backend / SSR**: Rust · [Leptos](https://leptos.dev) · Actix-Web
- **Frontend**: Leptos (WASM hydration) · TailwindCSS
- **Database**: PostgreSQL (SQLx migrations)
- **Cache**: Moka in-memory cache for auth lookups

## Quick Start

### Prerequisites

- Rust (stable) + [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos)
- Node.js (for TailwindCSS)
- PostgreSQL **or** Docker

### With Docker (recommended)

```bash
cp .env.example .env          # adjust values as needed
make docker-up                # starts app + db
```

### Local Development

```bash
cp .env.example .env          # adjust DATABASE_URL, SECRET_KEY, etc.
make docker-up-db             # start only the DB
make migrate                  # run pending SQL migrations
make dev                      # start dev server with live reload
```

Open [http://localhost:3000](http://localhost:3000).

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | — | PostgreSQL connection string |
| `SECRET_KEY` | — | Cookie signing key (≥ 64 bytes) |
| `ADMIN_USERNAME` | `admin` | Dashboard login |
| `ADMIN_PASSWORD` | `admin` | Dashboard password |
| `FREE_TRIAL_KEY` | `FREE_TRIAL` | Sending this as `auth_key` auto-creates a free-trial record |
| `AUTH_CACHE_TTL_SECONDS` | `900` | Auth result cache TTL (seconds) |

## Authentication API

`POST /v1/auth`

```json
{
  "auth_key": "YOUR_KEY",
  "auth_device": "DEVICE_ID",
  "rate_limit_usage": 1
}
```

| Status | Meaning |
|--------|---------|
| 200 | Key is valid; returns subscription info and remaining rate limit |
| 401 | Key invalid, not found, revoked, or expired |
| 429 | Rate limit exceeded |
| 500 | Internal server error |

Interactive docs: [http://localhost:3000/docs/](http://localhost:3000/docs/)

## Project Structure

```
src/
├── main.rs          Actix-Web server, /v1/auth endpoint, OpenAPI spec
├── app.rs           Leptos router
├── auth.rs          Admin session management
├── cache.rs         In-memory auth cache
├── db.rs            Database pool + migrations
├── models.rs        Core data models
├── components/      Reusable UI components
├── pages/           Page-level components
└── server/          Leptos server functions
migrations/          SQL migration files
agAuth/              Python auth microservice (standalone alternative)
```

## Makefile Targets

```
make dev          Start dev server (hot-reload)
make build        Production release build
make migrate      Run pending DB migrations
make fmt          Format code (cargo fmt)
make clippy       Run lints
make docker-up    Start all containers
make docker-down  Stop all containers
```
