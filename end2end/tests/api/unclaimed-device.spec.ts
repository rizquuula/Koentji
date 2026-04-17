// G3 — Unclaimed device sentinel: `device_id = '-'` rebinds on first call.
//
// Verified against src/infrastructure/postgres/issued_key_repository.rs
// ::claim_free_trial, Branch B. The rebind is NOT gated on the FREE_TRIAL
// marker — any key whose only row has device_id='-' gets adopted by the
// first caller. Flow:
//   1. find(key, real_dev) → None (no row matches)
//   2. claim_free_trial → Branch A skipped (key != marker) → Branch B
//      finds the (key, '-') row and UPDATEs it to real_dev
//   3. find(key, real_dev) → Some → use case proceeds to consume_quota
// After adoption the sentinel row no longer exists, so a second device
// calling the same key gets UnknownKey (401).
import { test, expect } from '@playwright/test';
import { withDb, insertKey, countKeysByKey } from '../../fixtures/db';

const PREFIX = 'e2e_envA_';
const KEY = `${PREFIX}unclaimed_key`;
const REAL_DEVICE = `${PREFIX}unclaimed_dev_real`;
const OTHER_DEVICE = `${PREFIX}unclaimed_dev_other`;

test.describe('POST /v1/auth — unclaimed sentinel adoption (G3)', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await c.query(
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_envA_%' OR device_id LIKE 'e2e_envA_%'",
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
        "DELETE FROM authentication_keys WHERE key LIKE 'e2e_envA_%' OR device_id LIKE 'e2e_envA_%' OR (key = $1 AND device_id = '-')",
        [KEY],
      ),
    );
  });

  test('first call with a real device rebinds the sentinel row', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: REAL_DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('success');
    expect(body.data.key).toBe(KEY);
    expect(body.data.device).toBe(REAL_DEVICE);
    expect(body.data.rate_limit_remaining).toBe(99);

    // Exactly one row for this key — the sentinel was UPDATEd, not a
    // duplicate inserted. device_id should now be the real device.
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
    expect(row.rate_limit_remaining).toBe(99);
  });

  test('second call with the adopted device decrements again', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: REAL_DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.rate_limit_remaining).toBe(98);

    const count = await withDb((c) => countKeysByKey(c, KEY));
    expect(count).toBe(1);
  });

  test('different device after adoption is rejected with 401 unknown', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: OTHER_DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    // UnknownKey envelope from i18n.rs::unknown_key.
    expect(body.error.en).toBe('Authentication key invalid or not exists in our system.');
    expect(body.error.id).toBe(
      'Authentication key tidak valid atau tidak ditemukan di sistem kami.',
    );

    // Still exactly one row — no phantom insert from the failed lookup.
    const count = await withDb((c) => countKeysByKey(c, KEY));
    expect(count).toBe(1);
  });
});
