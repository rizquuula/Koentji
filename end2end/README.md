# Koentji end-to-end tests

Playwright suite covering the Leptos dashboard UI, session auth, and the public `POST /v1/auth` API.

## One-time setup

```bash
make e2e-install
```

Installs the Playwright npm deps and the Chromium browser. You also need a running PostgreSQL on `127.0.0.1:5432` with the `koentji` superuser (the dev `docker-compose` DB works fine — the suite creates its own `koentjilab_test` database).

## Running locally

```bash
make e2e                       # full suite, cold-starts cargo leptos serve on :3001
cd end2end && npm run test:ui  # interactive Playwright UI
cd end2end && npx playwright test tests/auth/login.spec.ts --headed
```

The suite starts its own server on port **3001** so a `make dev` on port 3000 is unaffected.

### Environment overrides

Anything in `fixtures/env.ts` can be overridden. Useful ones:

| Var | Default | Purpose |
|---|---|---|
| `E2E_PORT` | `3001` | Port the test server binds to |
| `E2E_DATABASE_URL` | `postgres://koentji:koentji@127.0.0.1:5432/koentjilab_test` | Test DB URL |
| `E2E_ADMIN_USERNAME` / `E2E_ADMIN_PASSWORD` | `e2eadmin` / `e2eadmin` | Admin creds (passed to the test server as `ADMIN_USERNAME`/`ADMIN_PASSWORD`) |
| `E2E_FREE_TRIAL_KEY` | `FREE_TRIAL` | Free-trial magic key |

## How it works

1. `global-setup.ts` creates `koentjilab_test` (if missing), runs migrations via `cargo run --features ssr -- run-migrations`, seeds baseline rows, then logs in as admin and saves `storage/admin.json`.
2. Playwright's `webServer` config launches `cargo leptos serve` with the test env and waits for `http://127.0.0.1:3001` to respond.
3. Projects:
   - **chromium** — authenticated dashboard tests (reuses `storage/admin.json`).
   - **chromium-guest** — logged-out tests (`tests/auth/**`, `tests/smoke/**`).
   - **api** — `tests/api/**`, no browser, uses the Playwright `request` fixture.
   - **webkit-smoke** — `@smoke` tag only, WebKit engine, catches browser-specific hydration bugs.

## Adding a test

- **UI test touching the DB**: import from `fixtures/db.ts`, seed inside `beforeEach`/`beforeAll`, clean up in the matching `after` hook.
- **API test**: use the `request` fixture and hit `/v1/auth` directly.
- **Hydration / console-error check**: copy the `assertCleanHydration` helper in `tests/smoke/hydration.spec.ts`.

## Troubleshooting

- **Server never becomes ready**: first run can take 3+ minutes to compile Leptos in dev mode. The `webServer` timeout is 240s; raise it in `playwright.config.ts` if you're on a cold cache.
- **DB errors on global-setup**: ensure PostgreSQL is running and the `koentji` role can `CREATE DATABASE`. `make docker-up-db` starts the dev container with the right perms.
- **Port 3001 already in use**: set `E2E_PORT=3010` (or anything free).
- **Stale storageState**: delete `end2end/storage/admin.json` and re-run; `global-setup.ts` will rebuild it.
