// Pin the /v2/auth float-native envelope and confirm /v1/auth is unchanged.
//
// Field names verified against src/interface/http/auth_v2_endpoint.rs:
//   success body = { status: "success", data: { key, device, subscription,
//     username, email, valid_until, rate_limit_remaining: f64 } }
// v1 envelope (integer rate_limit_remaining) must keep its shape after a
// v2 hit on the same key.
import { test, expect } from '@playwright/test';
import { withDb, insertKey } from '../../fixtures/db';

const PREFIX = 'e2e_v2_';
const KEYS = {
  ok: `${PREFIX}auth_ok`,
  v1ParityProbe: `${PREFIX}v1_parity`,
  exhausted: `${PREFIX}exhausted`,
  negative: `${PREFIX}negative`,
};
const DEVICES = {
  ok: `${PREFIX}auth_ok_dev`,
  v1ParityProbe: `${PREFIX}v1_parity_dev`,
  exhausted: `${PREFIX}exhausted_dev`,
  negative: `${PREFIX}negative_dev`,
};

test.describe('POST /v2/auth — float-native envelope', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v2_%' OR device_id LIKE 'e2e_v2_%'",
      );
      await insertKey(c, {
        key: KEYS.ok,
        device_id: DEVICES.ok,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        username: 'v2-user',
        email: 'v2@e2e.test',
      });
      await insertKey(c, {
        key: KEYS.v1ParityProbe,
        device_id: DEVICES.v1ParityProbe,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      // Sub-1.0 remaining so a usage=1.0 request trips the atomic
      // consume's `remaining > usage` guard and yields 429.
      await insertKey(c, {
        key: KEYS.exhausted,
        device_id: DEVICES.exhausted,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 0.5,
      });
      await insertKey(c, {
        key: KEYS.negative,
        device_id: DEVICES.negative,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
    });
  });

  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v2_%' OR device_id LIKE 'e2e_v2_%'",
      ),
    );
  });

  test('401 unknown key returns the bilingual envelope', async ({ request }) => {
    const res = await request.post('/v2/auth', {
      data: {
        auth_key: `${PREFIX}definitely_unknown`,
        auth_device: `${PREFIX}nobody`,
        rate_limit_usage: 1.0,
      },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    expect(body.message).toBe(body.error.en);
  });

  test('fractional rate_limit_usage decrements by exactly that amount', async ({ request }) => {
    const first = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 0.5 },
    });
    expect(first.status()).toBe(200);
    const firstBody = await first.json();
    expect(firstBody.status).toBe('success');
    expect(typeof firstBody.data.rate_limit_remaining).toBe('number');
    const after = firstBody.data.rate_limit_remaining;

    const second = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 0.5 },
    });
    expect(second.status()).toBe(200);
    const secondBody = await second.json();
    // Float tolerance on the diff.
    expect(Math.abs(secondBody.data.rate_limit_remaining - (after - 0.5))).toBeLessThan(1e-9);
  });

  test('omitted rate_limit_usage consumes 1.0', async ({ request }) => {
    const before = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 1.0 },
    });
    const beforeRemaining = (await before.json()).data.rate_limit_remaining;

    const res = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(Math.abs(body.data.rate_limit_remaining - (beforeRemaining - 1.0))).toBeLessThan(1e-9);
  });

  test('rate_limit_usage = 0 is coerced to 1.0', async ({ request }) => {
    const before = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 1.0 },
    });
    const beforeRemaining = (await before.json()).data.rate_limit_remaining;

    const res = await request.post('/v2/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 0 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(Math.abs(body.data.rate_limit_remaining - (beforeRemaining - 1.0))).toBeLessThan(1e-9);
  });

  test('negative rate_limit_usage is coerced to 1.0', async ({ request }) => {
    const before = await request.post('/v2/auth', {
      data: { auth_key: KEYS.negative, auth_device: DEVICES.negative, rate_limit_usage: 1.0 },
    });
    const beforeRemaining = (await before.json()).data.rate_limit_remaining;

    const res = await request.post('/v2/auth', {
      data: { auth_key: KEYS.negative, auth_device: DEVICES.negative, rate_limit_usage: -3.5 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(Math.abs(body.data.rate_limit_remaining - (beforeRemaining - 1.0))).toBeLessThan(1e-9);
  });

  test('exhausted quota returns 429 with bilingual envelope', async ({ request }) => {
    const res = await request.post('/v2/auth', {
      data: { auth_key: KEYS.exhausted, auth_device: DEVICES.exhausted, rate_limit_usage: 1.0 },
    });
    expect(res.status()).toBe(429);
    const body = await res.json();
    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    expect(body.message).toBe(body.error.en);
    // Atomic consume must not have decremented the row on the 429 path.
    const remaining = await withDb(async (c) => {
      const { rows } = await c.query<{ rate_limit_remaining: number }>(
        'SELECT rate_limit_remaining FROM authentication_keys WHERE key = $1',
        [KEYS.exhausted],
      );
      return rows[0].rate_limit_remaining;
    });
    expect(Math.abs(remaining - 0.5)).toBeLessThan(1e-9);
  });

  test('v1 envelope still returns integer rate_limit_remaining', async ({ request }) => {
    // First hit v2 to prove the two endpoints share the same use case.
    const v2 = await request.post('/v2/auth', {
      data: {
        auth_key: KEYS.v1ParityProbe,
        auth_device: DEVICES.v1ParityProbe,
        rate_limit_usage: 1.0,
      },
    });
    expect(v2.status()).toBe(200);

    const v1 = await request.post('/v1/auth', {
      data: {
        auth_key: KEYS.v1ParityProbe,
        auth_device: DEVICES.v1ParityProbe,
        rate_limit_usage: 1,
      },
    });
    expect(v1.status()).toBe(200);
    const v1Body = await v1.json();
    expect(typeof v1Body.data.rate_limit_remaining).toBe('number');
    expect(Number.isInteger(v1Body.data.rate_limit_remaining)).toBe(true);
  });
});
