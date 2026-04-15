import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const KEYS = {
  revoked: 'klab_e2e_api_revoked_0001',
  expired: 'klab_e2e_api_expired_0001',
  free_trial_expired: 'klab_e2e_api_ft_expired_0001',
};

test.describe('POST /v1/auth — error paths', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      for (const k of Object.values(KEYS)) await deleteKeyByKey(c, k);

      await insertKey(c, {
        key: KEYS.revoked,
        device_id: 'api-device-revoked',
        subscription_type_name: 'basic',
        deleted_at: new Date(Date.now() - 86_400_000).toISOString(),
      });
      await insertKey(c, {
        key: KEYS.expired,
        device_id: 'api-device-expired',
        subscription_type_name: 'basic',
        expired_at: new Date(Date.now() - 3_600_000).toISOString(),
      });
      await insertKey(c, {
        key: KEYS.free_trial_expired,
        device_id: 'api-device-ft-expired',
        subscription_type_name: 'free',
        expired_at: new Date(Date.now() - 3_600_000).toISOString(),
      });
    });
  });

  test.afterAll(async () => {
    await withDb(async (c) => {
      for (const k of Object.values(KEYS)) await deleteKeyByKey(c, k);
    });
  });

  test('401 — unknown key', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: 'klab_nope_nope_nope_404', auth_device: 'nobody', rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.message).toMatch(/invalid or not exists/i);
  });

  test('401 — revoked key', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.revoked, auth_device: 'api-device-revoked', rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.message).toMatch(/already revoked/i);
  });

  test('401 — expired (non-trial) key', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEYS.expired, auth_device: 'api-device-expired', rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.message).toMatch(/expired and need renewal/i);
  });

  test('401 — free trial expired', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: {
        auth_key: KEYS.free_trial_expired,
        auth_device: 'api-device-ft-expired',
        rate_limit_usage: 1,
      },
    });
    expect(res.status()).toBe(401);
    const body = await res.json();
    expect(body.message).toMatch(/Free trial period has ended/i);
  });
});
