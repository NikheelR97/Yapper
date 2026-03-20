import { defineConfig, devices } from '@playwright/test';
import { existsSync } from 'fs';

const authStateFile = 'tests/auth-state.json';

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
	// Log in once; all authenticated tests reuse the saved refresh cookie
	globalSetup: process.env.E2E_EMAIL ? './tests/global-setup.ts' : undefined,
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: 1,
	reporter: process.env.CI
		? [
				['github'],
				['html', { outputFolder: 'playwright-report', open: 'never' }],
				// Allure for nightly artefact aggregation across shards.
				// Install: npm install --save-dev allure-playwright
				...(process.env.ALLURE === 'true'
					? [['allure-playwright', { outputFolder: 'allure-results', suiteTitle: true }] as const]
					: []),
		  ]
		: 'list',

	use: {
		baseURL: process.env.BASE_URL ?? 'http://localhost:5173',
		trace: process.env.CI ? 'retain-on-failure' : 'on-first-retry',
		screenshot: 'only-on-failure',
		video: process.env.CI ? 'retain-on-failure' : 'off',
		// HAR for network inspection — only on nightly (captures full request/response log)
		recordHar: process.env.NIGHTLY === 'true' ? { path: 'test-results/har/' } : undefined,
		// Reuse saved session so tests don't call POST /login individually
		storageState: process.env.E2E_EMAIL && existsSync(authStateFile)
			? authStateFile
			: undefined,
	},

	projects: [
		{
			name: 'chromium',
			use: {
				...devices['Desktop Chrome'],
				headless: !!process.env.CI,
			},
			testIgnore: ['**/mobile-responsive.spec.ts', '**/tauri-*.spec.ts', '**/accessibility.spec.ts'],
		},
		// Mobile Chrome — iPhone SE viewport (375×667).
		// Runs mobile-responsive.spec.ts + auth.spec.ts to validate layouts.
		{
			name: 'mobile-chrome',
			use: {
				...devices['iPhone SE'],
				headless: !!process.env.CI,
			},
			testMatch: ['**/mobile-responsive.spec.ts', '**/auth.spec.ts'],
		},
		// Tauri desktop — only runs when TAURI_BINARY env var is set (CI release builds).
		// Tests match tauri-*.spec.ts which guard themselves with __TAURI_INTERNALS__ detection.
		...(process.env.TAURI_BINARY
			? [
					{
						name: 'tauri-desktop',
						use: {
							...devices['Desktop Chrome'],
							baseURL: 'http://tauri.localhost',
							headless: false, // Tauri renders natively; headless not supported
						},
						testMatch: ['**/tauri-*.spec.ts'],
					},
			  ]
			: []),
	],

	webServer: process.env.TAURI_BINARY
		? {
				// Launch the Tauri binary; Playwright connects to the embedded WebView
				command: process.env.TAURI_BINARY,
				url: 'http://tauri.localhost',
				reuseExistingServer: false,
				timeout: 60_000,
			}
		: process.env.BASE_URL && !process.env.BASE_URL.includes('localhost')
			? undefined
			: {
					command: 'npm run dev',
					url: 'http://localhost:5173',
					reuseExistingServer: !process.env.CI,
					timeout: 120_000,
				},
});
