import path from 'node:path';

export const E2E_HOST = '127.0.0.1';
export const E2E_PORT = Number(process.env.E2E_PORT ?? 3001);
export const BASE_URL = `http://${E2E_HOST}:${E2E_PORT}`;

export const TEST_DB_NAME = process.env.E2E_DB_NAME ?? 'koentjilab_test';
export const TEST_DB_USER = process.env.E2E_DB_USER ?? 'koentji';
export const TEST_DB_PASSWORD = process.env.E2E_DB_PASSWORD ?? 'koentji';
export const TEST_DB_HOST = process.env.E2E_DB_HOST ?? '127.0.0.1';
export const TEST_DB_PORT = Number(process.env.E2E_DB_PORT ?? 5432);

export const DATABASE_URL =
  process.env.E2E_DATABASE_URL ??
  `postgres://${TEST_DB_USER}:${TEST_DB_PASSWORD}@${TEST_DB_HOST}:${TEST_DB_PORT}/${TEST_DB_NAME}`;

export const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'e2eadmin';
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? 'e2eadmin';

export const FREE_TRIAL_KEY = process.env.E2E_FREE_TRIAL_KEY ?? 'FREE_TRIAL';
export const FREE_TRIAL_SUBSCRIPTION_NAME = 'free';

export const SECRET_KEY =
  process.env.E2E_SECRET_KEY ??
  'koentji-e2e-secret-key-that-is-at-least-64-bytes-long-aaaaaaaaaaaaaaaaa';

export const STORAGE_STATE_PATH = path.resolve(__dirname, '..', 'storage', 'admin.json');
export const REPO_ROOT = path.resolve(__dirname, '..', '..');

export const SERVER_ENV: Record<string, string> = {
  DATABASE_URL,
  LEPTOS_SITE_ADDR: `${E2E_HOST}:${E2E_PORT}`,
  LEPTOS_RELOAD_PORT: String(E2E_PORT + 1),
  ADMIN_USERNAME,
  ADMIN_PASSWORD,
  FREE_TRIAL_KEY,
  FREE_TRIAL_SUBSCRIPTION_NAME,
  SECRET_KEY,
  AUTH_CACHE_TTL_SECONDS: '2',
  WORKERS: '2',
  RUST_LOG: 'info',
  // Playwright drives the server over plain HTTP; Secure cookies
  // would be dropped by the browser and the admin session would
  // never stick.
  COOKIE_SECURE: 'false',
};
