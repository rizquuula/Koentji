import { chromium, FullConfig, request } from '@playwright/test';
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { Client } from 'pg';
import {
  ADMIN_PASSWORD,
  ADMIN_USERNAME,
  BASE_URL,
  DATABASE_URL,
  REPO_ROOT,
  STORAGE_STATE_PATH,
  TEST_DB_HOST,
  TEST_DB_NAME,
  TEST_DB_PASSWORD,
  TEST_DB_PORT,
  TEST_DB_USER,
} from './fixtures/env';
import { insertKey, truncateKeys } from './fixtures/db';

function log(msg: string): void {
  // eslint-disable-next-line no-console
  console.log(`[e2e-setup] ${msg}`);
}

async function ensureDatabaseExists(): Promise<void> {
  const adminUrl = `postgres://${TEST_DB_USER}:${TEST_DB_PASSWORD}@${TEST_DB_HOST}:${TEST_DB_PORT}/postgres`;
  const admin = new Client({ connectionString: adminUrl });
  await admin.connect();
  try {
    const { rows } = await admin.query('SELECT 1 FROM pg_database WHERE datname = $1', [
      TEST_DB_NAME,
    ]);
    if (rows.length === 0) {
      log(`creating database ${TEST_DB_NAME}`);
      await admin.query(`CREATE DATABASE ${TEST_DB_NAME}`);
    } else {
      log(`database ${TEST_DB_NAME} already exists`);
    }
  } finally {
    await admin.end();
  }
}

function runMigrations(): void {
  log('running migrations (cargo run --features ssr -- run-migrations)');
  execSync('cargo run --features ssr -- run-migrations', {
    cwd: REPO_ROOT,
    stdio: 'inherit',
    env: { ...process.env, DATABASE_URL },
  });
}

async function seedBaseline(): Promise<void> {
  log('seeding baseline data');
  const c = new Client({ connectionString: DATABASE_URL });
  await c.connect();
  try {
    await truncateKeys(c);

    await insertKey(c, {
      key: 'klab_e2e_active_key_0001',
      device_id: 'e2e-device-active',
      subscription_type_name: 'free',
      rate_limit_daily: 100,
      rate_limit_remaining: 100,
      username: 'active-user',
      email: 'active@e2e.test',
    });

    await insertKey(c, {
      key: 'klab_e2e_revoked_key_0002',
      device_id: 'e2e-device-revoked',
      subscription_type_name: 'basic',
      rate_limit_daily: 100,
      rate_limit_remaining: 100,
      deleted_at: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    });

    await insertKey(c, {
      key: 'klab_e2e_expired_key_0003',
      device_id: 'e2e-device-expired',
      subscription_type_name: 'basic',
      rate_limit_daily: 100,
      rate_limit_remaining: 100,
      expired_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(),
    });
  } finally {
    await c.end();
  }
}

async function loginAndSaveStorage(): Promise<void> {
  log('logging in admin and saving storageState');
  const dir = path.dirname(STORAGE_STATE_PATH);
  fs.mkdirSync(dir, { recursive: true });

  const browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();

  await page.goto(`${BASE_URL}/login`);
  await page.getByPlaceholder('Enter your username').fill(ADMIN_USERNAME);
  await page.getByPlaceholder('Enter your password').fill(ADMIN_PASSWORD);
  await Promise.all([
    page.waitForURL(/\/dashboard/, { timeout: 30_000 }),
    page.getByRole('button', { name: /Sign In/i }).click(),
  ]);

  await context.storageState({ path: STORAGE_STATE_PATH });
  await browser.close();
}

export default async function globalSetup(_config: FullConfig): Promise<void> {
  await ensureDatabaseExists();
  runMigrations();
  await seedBaseline();
  await loginAndSaveStorage();
  log('global setup complete');
}
