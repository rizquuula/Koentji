// Pin the merged /v1/auth contract: fractional `rate_limit_usage` is
// accepted and decrements the ledger by exactly that amount, while the
// response envelope keeps its frozen integer `rate_limit_remaining`
// (ceil shim). Because the response no longer exposes the fractional
// remainder, the exact-decrement proof reads the DB ledger directly.
//
// Field names verified against src/interface/http/auth_endpoint.rs:
//   success body = { status: "success", data: { key, device, subscription,
//     username, email, valid_until, rate_limit_remaining: i32 } }
import { test, expect } from '@playwright/test';
import { withDb, insertKey, setRateLimitRemaining } from '../../fixtures/db';

const PREFIX = 'e2e_v1frac_';
const KEYS = {
  ok: `${PREFIX}auth_ok`,
  omitted: `${PREFIX}omitted`,
  zero: `${PREFIX}zero`,
  exhausted: `${PREFIX}exhausted`,
  negative: `${PREFIX}negative`,
};
const DEVICES = {
  ok: `${PREFIX}auth_ok_dev`,
  omitted: `${PREFIX}omitted_dev`,
  zero: `${PREFIX}zero_dev`,
  exhausted: `${PREFIX}exhausted_dev`,
  negative: `${PREFIX}negative_dev`,
};

// Read the fractional ledger value straight from Postgres — the /v1/auth
// response ceils it away, so this is the only place the exact remainder
// is observable.
async function ledgerRemaining(key: string): Promise<number> {
  return withDb(async (c) => {
    const { rows } = await c.query<{ rate_limit_remaining: number }>(
      'SELECT rate_limit_remaining FROM authentication_keys WHERE key = $1',
      [key],
    );
    return rows[0].rate_limit_remaining;
  });
}

test.describe('POST /v1/auth — fractional usage, integer envelope', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1frac_%' OR device_id LIKE 'e2e_v1frac_%'",
      );
      await insertKey(c, {
        key: KEYS.ok,
        device_id: DEVICES.ok,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        username: 'v1frac-user',
        email: 'v1frac@e2e.test',
      });
      await insertKey(c, {
        key: KEYS.omitted,
        device_id: DEVICES.omitted,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      await insertKey(c, {
        key: KEYS.zero,
        device_id: DEVICES.zero,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      // Sub-1.0 remaining so a usage=1.0 request trips the atomic
      // consume's `remaining > usage` guard and yields 429.
      // setRateLimitRemaining also stamps rate_limit_updated_at so the
      // interval-reset branch doesn't refill the bucket on first hit.
      const exhausted = await insertKey(c, {
        key: KEYS.exhausted,
        device_id: DEVICES.exhausted,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
      await setRateLimitRemaining(c, exhausted.id, 0.5);
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
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1frac_%' OR device_id LIKE 'e2e_v1frac_%'",
      ),
    );
  });

  test('401 unknown key returns the bilingual envelope', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: {
        auth_key: `${PREFIX}definitely_unknown`,
        auth_device: `${PREFIX}nobody`,
        rate_limit_usage: 1,
      },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    expect(body.message).toBe(body.error.en);
  });

  test('fractional rate_limit_usage decrements the ledger by exactly that amount', async ({
    request,
  }) => {
    const before = await ledgerRemaining(KEYS.ok);

    const first = await request.post('/v1/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 0.5 },
    });
    expect(first.status()).toBe(200);
    const firstBody = await first.json();
    expect(firstBody.status).toBe('success');
    // Frozen envelope: integer remaining (ceil shim), never the raw float.
    expect(Number.isInteger(firstBody.data.rate_limit_remaining)).toBe(true);
    expect(Math.abs((await ledgerRemaining(KEYS.ok)) - (before - 0.5))).toBeLessThan(1e-9);

    const second = await request.post('/v1/auth', {
      data: { auth_key: KEYS.ok, auth_device: DEVICES.ok, rate_limit_usage: 0.5 },
    });
    expect(second.status()).toBe(200);
    expect(Math.abs((await ledgerRemaining(KEYS.ok)) - (before - 1.0))).toBeLessThan(1e-9);
  });

  test('omitted rate_limit_usage consumes 1.0', async ({ request }) => {
    const before = await ledgerRemaining(KEYS.omitted);
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.omitted, auth_device: DEVICES.omitted },
    });
    expect(res.status()).toBe(200);
    expect(Math.abs((await ledgerRemaining(KEYS.omitted)) - (before - 1.0))).toBeLessThan(1e-9);
  });

  test('rate_limit_usage = 0 is coerced to 1.0', async ({ request }) => {
    const before = await ledgerRemaining(KEYS.zero);
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.zero, auth_device: DEVICES.zero, rate_limit_usage: 0 },
    });
    expect(res.status()).toBe(200);
    expect(Math.abs((await ledgerRemaining(KEYS.zero)) - (before - 1.0))).toBeLessThan(1e-9);
  });

  test('negative rate_limit_usage is coerced to 1.0', async ({ request }) => {
    const before = await ledgerRemaining(KEYS.negative);
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.negative, auth_device: DEVICES.negative, rate_limit_usage: -3.5 },
    });
    expect(res.status()).toBe(200);
    expect(Math.abs((await ledgerRemaining(KEYS.negative)) - (before - 1.0))).toBeLessThan(1e-9);
  });

  test('exhausted quota returns 429 with bilingual envelope and no decrement', async ({
    request,
  }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.exhausted, auth_device: DEVICES.exhausted, rate_limit_usage: 1.0 },
    });
    expect(res.status()).toBe(429);
    const body = await res.json();
    expect(typeof body.error.en).toBe('string');
    expect(typeof body.error.id).toBe('string');
    expect(body.message).toBe(body.error.en);
    // Atomic consume must not have decremented the row on the 429 path.
    expect(Math.abs((await ledgerRemaining(KEYS.exhausted)) - 0.5)).toBeLessThan(1e-9);
  });
});
