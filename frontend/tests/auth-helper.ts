import { type Page } from '@playwright/test';
import { existsSync, readFileSync } from 'fs';

interface AuthData {
	accessToken: string;
	csrfToken: string;
	user: Record<string, unknown>;
}

/**
 * Load saved auth data from global setup.
 * Returns null if auth-data.json doesn't exist yet (first run / no E2E_EMAIL set).
 */
export function loadAuthData(): AuthData | null {
	const path = 'tests/auth-data.json';
	if (!existsSync(path)) return null;
	try {
		return JSON.parse(readFileSync(path, 'utf-8')) as AuthData;
	} catch {
		return null;
	}
}

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

/**
 * Mock the auth refresh and users/me endpoints so each test doesn't burn a
 * rate-limit token. The app's (app)/+layout calls these on every page load to
 * restore session; without mocking, 5+ tests from the same IP hit the
 * burst limit (20 req) and get 429, causing redirects to /login.
 *
 * Call this before page.goto() in loginAs / beforeEach.
 */
export async function mockAuthEndpoints(page: Page): Promise<void> {
	const data = loadAuthData();
	if (!data) return; // no saved data — let real calls through

	const { accessToken, csrfToken, user } = data;

	// Mock POST /api/v1/auth/refresh — return cached token without hitting backend
	await page.route(`${API_URL}/api/v1/auth/refresh`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ access_token: accessToken, csrf_token: csrfToken }),
		});
	});

	// Mock GET /api/v1/users/me — return cached user object
	await page.route(`${API_URL}/api/v1/users/me`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify(user),
		});
	});
}
