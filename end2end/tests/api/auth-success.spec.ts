import { test, expect } from '@playwright/test';
import { withDb, insertKey, deleteKeyByKey } from '../../fixtures/db';

const KEY = 'klab_e2e_api_success_0001';
const DEVICE = 'api-success-device';

test.describe('POST /v1/auth — success', () => {
  test.beforeAll(async () => {
    await withDb(async (c) => {
      await deleteKeyByKey(c, KEY);
      await insertKey(c, {
        key: KEY,
        device_id: DEVICE,
        subscription_type_name: 'free',
        rate_limit_daily: 100,
        rate_limit_remaining: 100,
        username: 'api-user',
        email: 'api@e2e.test',
      });
    });
  });

  test.afterAll(async () => {
    await withDb((c) => deleteKeyByKey(c, KEY));
  });

  test('200 with success envelope and decrements rate_limit_remaining', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();

    expect(body.status).toBe('success');
    expect(body.data).toMatchObject({
      key: KEY,
      device: DEVICE,
      subscription: 'free',
      username: 'api-user',
      email: 'api@e2e.test',
    });
    expect(typeof body.data.rate_limit_remaining).toBe('number');
    expect(body.data.rate_limit_remaining).toBe(99);
  });

  test('second call hits cache and returns consistent shape', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE, rate_limit_usage: 2 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.rate_limit_remaining).toBeLessThan(100);
  });

  test('default rate_limit_usage = 1 when omitted', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: KEY, auth_device: DEVICE },
    });
    expect(res.status()).toBe(200);
  });
});
