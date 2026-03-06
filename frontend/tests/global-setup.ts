import { chromium, type FullConfig } from '@playwright/test';
import { writeFileSync } from 'fs';

/**
 * Global setup: logs in ONCE and saves:
 *  - tests/auth-state.json  — browser cookies (HttpOnly refresh_token)
 *  - tests/auth-data.json   — access_token, csrf_token, user object
 *
 * Individual tests mock POST /auth/refresh and GET /users/me using the saved
 * data so they don't burn through the backend's per-IP rate limit (burst 20,
 * 100/min). Only the single global login call hits the real login endpoint.
 */
export default async function globalSetup(config: FullConfig) {
	const email = process.env.E2E_EMAIL;
	const password = process.env.E2E_PASSWORD;

	if (!email || !password) {
		return;
	}

	const baseURL = config.projects[0]?.use?.baseURL ?? 'http://localhost:5173';
	const apiURL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

	const browser = await chromium.launch();
	const context = await browser.newContext({ baseURL });
	const page = await context.newPage();

	// Intercept the login response to capture access_token + csrf_token
	let accessToken = '';
	let csrfToken = '';

	await page.route(`${apiURL}/api/v1/auth/login`, async (route) => {
		const response = await route.fetch();
		const body = await response.json().catch(() => ({}));
		accessToken = body.access_token ?? '';
		csrfToken = body.csrf_token ?? '';
		await route.fulfill({ response });
	});

	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 30_000 });

	// Fetch user profile with the captured access token
	let user = {};
	if (accessToken) {
		const res = await page.request.get(`${apiURL}/api/v1/users/me`, {
			headers: { Authorization: `Bearer ${accessToken}` },
		});
		user = await res.json().catch(() => ({}));
	}

	// Save browser state (cookies) and auth data
	await context.storageState({ path: 'tests/auth-state.json' });
	writeFileSync(
		'tests/auth-data.json',
		JSON.stringify({ accessToken, csrfToken, user }, null, 2),
	);

	await browser.close();
}
