// Request-id middleware (G8):
//   - absent inbound → server mints a UUIDv7 and emits it
//   - trusted inbound (printable ASCII, ≤128 bytes) → echoed verbatim
//   - invalid inbound (>128 bytes, per MAX_INBOUND_LEN in request_id.rs) →
//     replaced with a freshly minted UUIDv7
//   - inbound id survives over POST (endpoint-agnostic middleware)
//
// Control-character rejection is enforced in the Rust unit test
// (`rejects_inbound_with_control_chars`) because Playwright's `request`
// API can't easily smuggle raw tabs/newlines past the HTTP parser.

import { test, expect } from '@playwright/test';

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

test.describe('X-Request-Id middleware', () => {
  test('mints a UUIDv7 when the inbound header is absent', async ({ request }) => {
    const res = await request.get('/healthz');
    const rid = res.headers()['x-request-id'];
    expect(rid).toBeDefined();
    expect(rid).toHaveLength(36);
    expect(rid).toMatch(UUID_V7);
  });

  test('echoes a valid inbound X-Request-Id verbatim', async ({ request }) => {
    const inbound = 'test-rid-cluster-c-abc123';
    const res = await request.get('/healthz', {
      headers: { 'X-Request-Id': inbound },
    });
    expect(res.headers()['x-request-id']).toBe(inbound);
  });

  test('rejects an oversized (>128 byte) inbound id and mints a fresh UUIDv7', async ({
    request,
  }) => {
    // MAX_INBOUND_LEN is 128 in src/infrastructure/telemetry/request_id.rs;
    // 129 'a's should trip the length guard.
    const oversized = 'a'.repeat(129);
    const res = await request.get('/healthz', {
      headers: { 'X-Request-Id': oversized },
    });
    const rid = res.headers()['x-request-id'];
    expect(rid).toBeDefined();
    expect(rid).not.toBe(oversized);
    expect(rid).toMatch(UUID_V7);
  });

  test('echoes inbound id on POST /v1/auth regardless of auth outcome', async ({ request }) => {
    const inbound = 'test-rid-post-abc';
    const res = await request.post('/v1/auth', {
      headers: { 'X-Request-Id': inbound },
      data: {
        auth_key: 'klab_nope_nope_nope_404',
        auth_device: 'nobody',
        rate_limit_usage: 1,
      },
    });
    // The envelope will be a 401 here (unknown key), but the request-id
    // middleware runs regardless — the header must still be mirrored.
    expect(res.headers()['x-request-id']).toBe(inbound);
  });
});
