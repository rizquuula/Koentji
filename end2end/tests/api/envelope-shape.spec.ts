// G1 — Pin the /v1/auth envelope shape.
//
// Field names verified against src/interface/http/auth_endpoint.rs:
//   success body = { status: "success", data: { key, device, subscription,
//     username, email, valid_until, rate_limit_remaining } }
// Error strings verified against src/interface/http/i18n.rs (byte-identical).
import { test, expect } from '@playwright/test';
import { withDb, insertKey } from '../../fixtures/db';

const PREFIX = 'e2e_envA_';
const KEYS = {
  ok: `${PREFIX}shape_ok`,
  revoked: `${PREFIX}shape_revoked`,
  rateLimited: `${PREFIX}shape_rl`,
};
const DEVICES = {
  ok: `${PREFIX}shape_ok_dev`,
  revoked: `${PREFIX}shape_revoked_dev`,
  rateLimited: `${PREFIX}shape_rl_dev`,
};

test.describe('POST /v1/auth — envelope shape (G1)', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_envA_%' OR device_id LIKE 'e2e_envA_%'",
      );
      await insertKey(c, {
        key: KEYS.ok,
        device_id: DEVICES.ok,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        username: 'envA-user',
        email: 'envA@e2e.test',
      });
      await insertKey(c, {
        key: KEYS.revoked,
        device_id: DEVICES.revoked,
        subscription_type_name: 'basic',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        deleted_at: new Date(Date.now() - 3_600_000).toISOString(),
      });
      await insertKey(c, {
        key: KEYS.rateLimited,
        device_id: DEVICES.rateLimited,
        subscription_type_name: 'basic',
        rate_limit_daily: 100,
        rate_limit_remaining: 0,
      });
      // Pin rate_limit_updated_at to "now" so the atomic consume SQL does
      // NOT treat the window as elapsed and refill remaining to daily.
      await c.query(
        "UPDATE authentication_keys SET rate_limit_updated_at = NOW() WHERE key = $1",
        [KEYS.rateLimited],
      );
    });
  });

  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_envA_%' OR device_id LIKE 'e2e_envA_%'",
      ),
    );
  });

  test('200 success envelope exposes the frozen field set', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();

    expect(body.status).toBe('success');
    expect(body.data).toBeDefined();
    expect(body.data.key).toBe(KEYS.ok);
    expect(body.data.device).toBe(DEVICES.ok);
    expect(typeof body.data.subscription).toBe('string');
    expect(body.data.subscription).toBe('free');
    expect(typeof body.data.rate_limit_remaining).toBe('number');
    expect(body.data.rate_limit_remaining).toBe(99);
    // valid_until is string | null (RFC3339 when set). Key seeded without
    // expired_at, so it must be null here.
    expect(body.data.valid_until).toBeNull();
    // username/email echoed through from the row.
    expect(body.data.username).toBe('envA-user');
    expect(body.data.email).toBe('envA@e2e.test');
  });

  test('401 unknown key returns bilingual non-empty envelope', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: {
        auth_key: `${PREFIX}definitely_unknown`,
        auth_device: `${PREFIX}nobody`,
        rate_limit_usage: 1,
      },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();

    expect(body.error).toBeDefined();
    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    expect(body.error.en.length).toBeGreaterThan(0);
    expect(body.error.id.length).toBeGreaterThan(0);
    expect(body.error.en).not.toBe(body.error.id);
    expect(body.error.en).toBe('Authentication key invalid or not exists in our system.');
    expect(body.error.id).toBe(
      'Authentication key tidak valid atau tidak ditemukan di sistem kami.',
    );
    expect(body.message).toBe(body.error.en);
  });

  test('401 revoked key renders the revoked bilingual strings', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: {
        auth_key: KEYS.revoked,
        auth_device: DEVICES.revoked,
        rate_limit_usage: 1,
      },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();

    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    // Prefix + suffix from i18n.rs::revoked — the middle carries the
    // revocation timestamp which varies per run.
    expect(body.error.en).toMatch(/^Authentication key already revoked and can't be used since /);
    expect(body.error.en).toMatch(/\.$/);
    expect(body.error.id).toMatch(/^Authentication key sudah tidak bisa digunakan sejak /);
    expect(body.error.id).toMatch(/\.$/);
    expect(body.message).toBe(body.error.en);
  });

  test('429 rate-limit exceeded renders the rate-limit bilingual strings', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: {
        auth_key: KEYS.rateLimited,
        auth_device: DEVICES.rateLimited,
        rate_limit_usage: 1,
      },
    });
    expect(res.status()).toBe(429);
    const body = await res.json();

    expect(body.error.en).toBe(
      'Rate limit exceeded. Please try again later or upgrade your subscription.',
    );
    expect(body.error.id).toBe(
      'Batas rate limit terlampaui. Silakan coba lagi nanti atau upgrade langganan Anda.',
    );
    expect(body.message).toBe(body.error.en);
  });
});
