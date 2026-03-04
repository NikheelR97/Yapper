import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E test configuration.
 *
 * Run:  npm run test:e2e
 * UI:   npx playwright test --ui
 *
 * Tests run against the SvelteKit dev server (port 5173) by default.
 * Set BASE_URL env var to point at a different environment (e.g. staging).
 *
 * Required env vars for auth tests (create a .env.test file or set in CI):
 *   E2E_EMAIL     — email of a pre-existing test account
 *   E2E_PASSWORD  — password for that account
 */
export default defineConfig({
	testDir: './tests',
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 1 : 0,
	workers: 1,
	reporter: process.env.CI ? 'github' : 'list',

	use: {
		baseURL: process.env.BASE_URL ?? 'http://localhost:5173',
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
	},

	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] },
		},
	],

	webServer: {
		command: 'npm run dev',
		url: 'http://localhost:5173',
		reuseExistingServer: !process.env.CI,
		timeout: 120_000,
	},
});
