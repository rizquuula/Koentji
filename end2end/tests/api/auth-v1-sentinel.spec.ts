// Unclaimed device sentinel adoption via /v1/auth, exercised with
// fractional usage.
//
// Mirrors unclaimed-device.spec.ts (G3) but drives a fractional consume
// through the merged endpoint. The HTTP envelope ceils `rate_limit_remaining`
// to an integer, so the exact post-decrement remainder (99.75) is asserted
// against the Postgres ledger, not the response body.
import { test, expect } from '@playwright/test';
import { withDb, insertKey, countKeysByKey } from '../../fixtures/db';

const PREFIX = 'e2e_v1sent_';
const KEY = `${PREFIX}unclaimed_key`;
const REAL_DEVICE = `${PREFIX}unclaimed_dev_real`;
const OTHER_DEVICE = `${PREFIX}unclaimed_dev_other`;

test.describe('POST /v1/auth — unclaimed sentinel adoption', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1sent_%' OR device_id LIKE 'e2e_v1sent_%'",
      );
      await insertKey(c, {
        key: KEY,
        device_id: '-',
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
      });
    });
  });

  test.afterAll(async () => {
    await withDb((c) =>
      c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_v1sent_%' OR device_id LIKE 'e2e_v1sent_%' OR (key = $1 AND device_id = '-')",
        [KEY],
      ),
    );
  });

  test('first /v1/auth call with a real device rebinds the sentinel row', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: REAL_DEVICE, rate_limit_usage: 0.25 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('success');
    expect(body.data.key).toBe(KEY);
    expect(body.data.device).toBe(REAL_DEVICE);
    // Frozen envelope: integer remaining via ceil shim. ceil(99.75) = 100.
    expect(body.data.rate_limit_remaining).toBe(100);

    const count = await withDb((c) => countKeysByKey(c, KEY));
    expect(count).toBe(1);

    const row = await withDb(async (c) => {
      const { rows } = await c.query<{ device_id: string; rate_limit_remaining: number }>(
        'SELECT device_id, rate_limit_remaining FROM authentication_keys WHERE key = $1',
        [KEY],
      );
      return rows[0];
    });
    expect(row.device_id).toBe(REAL_DEVICE);
    // The ledger keeps the exact fractional remainder the envelope ceils away.
    expect(Math.abs(row.rate_limit_remaining - 99.75)).toBeLessThan(1e-9);
  });

  test('different device after adoption is rejected with 401 unknown', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: OTHER_DEVICE, rate_limit_usage: 1.0 },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.error.en).toBe('Authentication key invalid or not exists in our system.');
    expect(body.error.id).toBe(
      'Authentication key tidak valid atau tidak ditemukan di sistem kami.',
    );

    const count = await withDb((c) => countKeysByKey(c, KEY));
    expect(count).toBe(1);
  });
});
