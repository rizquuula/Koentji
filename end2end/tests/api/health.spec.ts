// Health probes (G7):
//   - /healthz is pure liveness (no pool touched) → returns {"status":"ok"}
//   - /readyz runs `SELECT 1` with a 2s timeout → {"status":"ready","database":"ok"} on 200
//
// The 503 / pool-down path is NOT exercised here: the live e2e server shares
// its pool with the rest of the suite, and breaking it would cascade into
// unrelated failures. Coverage for the failure branch lives in the Rust
// integration test `tests/health_endpoints.rs`.

import { test, expect } from '@playwright/test';

test.describe('health probes', () => {
  test('/healthz returns 200 with {"status":"ok"} and does not touch the DB', async ({
    request,
  }) => {
    const res = await request.get('/healthz');
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('ok');
    // Liveness must not leak pool state — the payload is exactly {status: "ok"}.
    expect(Object.keys(body)).toEqual(['status']);
  });

  test('/healthz does not require a session cookie', async ({ request }) => {
    // The api project is already cookieless — asserting 200 here pins the
    // "public probe, no auth" contract so a future middleware addition
    // can't silently gate liveness behind the admin session.
    const res = await request.get('/healthz');
    expect(res.status()).toBe(200);
  });

  test('/readyz returns 200 with database=ok when the pool is healthy', async ({ request }) => {
    const res = await request.get('/readyz');
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('ready');
    expect(body.database).toBe('ok');
  });
});
