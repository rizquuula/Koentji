// Free trials are gone: /v1/auth never provisions a licence.
//
// The repository used to auto-insert a row when `auth_key` matched the
// documented `FREE_TRIAL` marker. That was an unauthenticated way to
// mint a working licence, so the branch was removed. What remains is
// src/infrastructure/postgres/issued_key_repository.rs::claim_unclaimed_key,
// which only binds an *admin-issued* key whose device_id is still the
// '-' sentinel.
//
// This spec pins the regression: the former magic key is now just an
// unknown key — 401 with the frozen UnknownKey envelope, and no row is
// written to the database.
import { test, expect } from '@playwright/test';
import { countKeysByDevice, countKeysByKey, withDb } from '../../fixtures/db';

const MARKER = 'FREE_TRIAL';
const DEVICE = `e2e-no-trial-device-${Date.now()}`;

test.describe('POST /v1/auth — no self-service provisioning', () => {
  test.afterAll(async () => {
    await withDb((c) =>
      c.query('DELETE FROM authentication_keys WHERE key = $1 OR device_id = $2', [MARKER, DEVICE]),
    );
  });

  test('the retired FREE_TRIAL marker is denied and provisions nothing', async ({ request }) => {
    expect(await withDb((c) => countKeysByDevice(c, DEVICE))).toBe(0);

    const res = await request.post('/v1/auth', {
      data: { auth_key: MARKER, auth_device: DEVICE, rate_limit_usage: 1 },
    });

    expect(res.status()).toBe(401);
    const body = await res.json();
    // UnknownKey envelope from i18n.rs::unknown_key — unchanged.
    expect(body.error.en).toBe('Authentication key invalid or not exists in our system.');
    expect(body.error.id).toBe(
      'Authentication key tidak valid atau tidak ditemukan di sistem kami.',
    );

    // Nothing was written for either the marker or the device.
    expect(await withDb((c) => countKeysByDevice(c, DEVICE))).toBe(0);
    expect(await withDb((c) => countKeysByKey(c, MARKER))).toBe(0);
  });

  test('a second call is still denied — no row appeared in between', async ({ request }) => {
    const res = await request.post('/v1/auth', {
      data: { auth_key: MARKER, auth_device: DEVICE, rate_limit_usage: 1 },
    });
    expect(res.status()).toBe(401);
    expect(await withDb((c) => countKeysByDevice(c, DEVICE))).toBe(0);
  });
});
