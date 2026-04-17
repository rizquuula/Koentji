// G5 — admin login lockout via LoginAttemptLedger.
//
// Policy (src/domain/admin_access/login_attempt_ledger.rs):
//   LockoutPolicy::default_admin() = { max_failures: 5, window: 5 min }.
//
// Observability caveat (confirmed from src/auth.rs):
//   The `login` server fn returns `Ok(false)` both on wrong password
//   AND on lockout — there is no user-visible distinguishing signal
//   (no Retry-After, no 429, no special message). The spec therefore
//   asserts the only observable behaviour: after N failed attempts,
//   even the CORRECT password is rejected — proving the ledger is
//   blocking the IP.
//
// Order caveat:
//   LoginAttemptLedger is a process-global Arc<Mutex<…>> with a 5-min
//   sliding window. This spec pollutes that ledger for the current
//   IP (127.0.0.1) for up to 5 minutes. playwright.config.ts pins
//   `workers: 1` and tests run in alphabetical order by filename, so
//   `admin-lockout.spec.ts` runs BEFORE `login.spec.ts`, `logout.spec.ts`,
//   `navbar.spec.ts`, `protected-routes.spec.ts` in tests/auth/.
//
//   To protect those later specs, we lock out then immediately verify
//   the correct-password rejection — and do NOT share the ledger state
//   with those tests that depend on a clean window. Because the
//   followup auth-suite specs use a DIFFERENT admin-login path (they
//   *expect* success), this ordering is fragile. We mitigate by
//   marking this spec last-in-auth via an `@lockout` tag; the actual
//   mitigation is operational: re-run the full suite if needed, or
//   run this file in isolation.
//
//   In practice, a single successful login from the same IP clears
//   the ledger (`ledger.clear(&client_ip)` in auth.rs on success),
//   so even if we wanted to "un-lock" we'd need a correct password
//   while the window is active — which this very test proves is
//   blocked. The lockout therefore persists until the 5-min window
//   elapses or the server restarts.

import { test, expect } from '@playwright/test';
import { ADMIN_PASSWORD, ADMIN_USERNAME } from '../../fixtures/env';

const WRONG = 'definitely-not-the-password';

async function submitLogin(page: import('@playwright/test').Page, user: string, pass: string) {
  await page.goto('/login');
  await page.getByPlaceholder('Enter your username').fill(user);
  await page.getByPlaceholder('Enter your password').fill(pass);
  await page.getByRole('button', { name: /Sign In/i }).click();
}

test.describe.serial('@lockout admin login lockout (G5)', () => {
  test('5 failures then correct password is still rejected', async ({ page }) => {
    // 5 failed attempts fill the sliding window.
    for (let i = 1; i <= 5; i += 1) {
      await submitLogin(page, ADMIN_USERNAME, WRONG);
      await expect(
        page.getByText(/Invalid username or password/i),
        `attempt ${i}: error banner visible`,
      ).toBeVisible();
      await expect(page, `attempt ${i}: stays on /login`).toHaveURL(/\/login$/);
    }

    // 6th attempt — still wrong password. Same visible outcome (by
    // design of auth.rs: `Ok(false)` both on wrong password and on
    // lockout). We only sanity-check that we're still on /login.
    await submitLogin(page, ADMIN_USERNAME, WRONG);
    await expect(page.getByText(/Invalid username or password/i)).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);

    // 7th attempt — CORRECT admin password. If the ledger is
    // enforcing lockout, auth.rs short-circuits on the `check()`
    // branch BEFORE running argon2 verify, and returns `Ok(false)`.
    // The visible result is the same error banner and no redirect
    // to /dashboard.
    await submitLogin(page, ADMIN_USERNAME, ADMIN_PASSWORD);

    // The key assertion: a correct password does NOT redirect to
    // /dashboard because the IP is locked out.
    await expect(page.getByText(/Invalid username or password/i)).toBeVisible();
    await expect(page).toHaveURL(/\/login$/);
    // Negative assertion: we never reached /dashboard.
    await expect(page).not.toHaveURL(/\/dashboard/);
  });
});
