import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';
import { BASE_URL, REPO_ROOT, SERVER_ENV, STORAGE_STATE_PATH } from './fixtures/env';

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [['list'], ['html', { open: 'never' }]],
  timeout: 60_000,
  expect: { timeout: 10_000 },

  use: {
    baseURL: BASE_URL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
  },

  globalSetup: require.resolve('./global-setup'),

  projects: [
    {
      name: 'setup',
      testMatch: /.*\.setup\.ts/,
    },
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        storageState: STORAGE_STATE_PATH,
      },
      testIgnore: [/tests\/auth\/.*/, /tests\/smoke\/.*/, /tests\/api\/.*/],
    },
    {
      name: 'chromium-guest',
      use: {
        ...devices['Desktop Chrome'],
        storageState: { cookies: [], origins: [] },
      },
      testMatch: [/tests\/auth\/.*/, /tests\/smoke\/.*/],
    },
    {
      name: 'api',
      use: {
        baseURL: BASE_URL,
        storageState: { cookies: [], origins: [] },
      },
      testMatch: /tests\/api\/.*/,
    },
    {
      name: 'webkit-smoke',
      use: {
        ...devices['Desktop Safari'],
        storageState: { cookies: [], origins: [] },
      },
      testMatch: /tests\/smoke\/.*/,
      grep: /@smoke/,
    },
  ],

  webServer: {
    command: 'cargo leptos serve',
    cwd: REPO_ROOT,
    url: BASE_URL,
    timeout: 240_000,
    reuseExistingServer: !process.env.CI,
    stdout: 'pipe',
    stderr: 'pipe',
    env: SERVER_ENV,
  },
});
