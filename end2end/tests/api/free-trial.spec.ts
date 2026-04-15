import { test, expect } from '@playwright/test';
import { FREE_TRIAL_KEY } from '../../fixtures/env';
import { countKeysByDevice, deleteKeyByDevice, withDb } from '../../fixtures/db';

const DEVICE = `e2e-free-trial-device-${Date.now()}`;

test.describe('POST /v1/auth — free trial auto-provisioning', () => {
  test.afterAll(async () => {
    await withDb((c) => deleteKeyByDevice(c, DEVICE));
  });

  test('first call with FREE_TRIAL_KEY provisions a row for the device', async ({ request }) => {
    const before = await withDb((c) => countKeysByDevice(c, DEVICE));
    expect(before).toBe(0);

    const res = await request.post('/v1/auth', {
      data: { auth_key: FREE_TRIAL_KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('success');
    expect(body.data.device).toBe(DEVICE);
    expect(body.data.subscription).toBe('free');

    const after = await withDb((c) => countKeysByDevice(c, DEVICE));
    expect(after).toBe(1);
  });

  test('second call reuses the same row (idempotent)', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: FREE_TRIAL_KEY, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(200);
    const count = await withDb((c) => countKeysByDevice(c, DEVICE));
    expect(count).toBe(1);
  });
});
