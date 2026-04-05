import { test, expect } from '@playwright/test';
import { loginViaApi } from './auth-helper.js';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_PASS = process.env.E2E_PASSWORD ?? '';

test.describe('Refresh token replay @security @auth', () => {
	test.skip(!USER_EMAIL || !USER_PASS, 'Set E2E_EMAIL / E2E_PASSWORD to run refresh replay tests');

	test('reusing the same refresh token concurrently succeeds once and rejects the replay', async () => {
		const session = await loginViaApi(USER_EMAIL, USER_PASS, {
			installationId: crypto.randomUUID(),
			label: 'Refresh Replay',
			platform: 'tauri',
		});
		expect(session.refreshToken).toBeTruthy();

		const refresh = () =>
			fetch(`${API_URL}/api/v2/auth/refresh`, {
				method: 'POST',
				headers: { 'x-refresh-token': session.refreshToken! },
			});
		const [first, second] = await Promise.all([refresh(), refresh()]);
		const statuses = [first.status, second.status].sort((a, b) => a - b);

		expect(statuses[0]).toBe(200);
		expect([401, 403]).toContain(statuses[1]);
	});
});
