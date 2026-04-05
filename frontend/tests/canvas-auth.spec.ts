import { test, expect } from '@playwright/test';
import { loginViaApi, PRIMARY_INSTALLATION_ID, B_PRIMARY_INSTALLATION_ID } from './auth-helper.js';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_A_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_A_PASS = process.env.E2E_PASSWORD ?? '';
const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';

function headers(session: Awaited<ReturnType<typeof loginViaApi>>) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

test.describe('Canvas auth @security @regression', () => {
	test.skip(
		!USER_A_EMAIL || !USER_A_PASS || !USER_B_EMAIL || !USER_B_PASS,
		'Set both E2E accounts to run canvas auth tests',
	);

	test('non-admin members cannot enqueue tracks through the canvas API directly', async () => {
		const owner = await loginViaApi(USER_A_EMAIL, USER_A_PASS, {
			installationId: PRIMARY_INSTALLATION_ID,
			label: 'Canvas Owner',
		});
		const member = await loginViaApi(USER_B_EMAIL, USER_B_PASS, {
			installationId: B_PRIMARY_INSTALLATION_ID,
			label: 'Canvas Member',
		});

		const createServer = await fetch(`${API_URL}/api/v2/servers`, {
			method: 'POST',
			headers: headers(owner),
			body: JSON.stringify({ name: `Canvas Auth ${Date.now()}` }),
		});
		expect(createServer.ok).toBeTruthy();
		const server = (await createServer.json()) as { id: string };
		const createInvite = await fetch(`${API_URL}/api/v2/servers/${server.id}/invite`, {
			method: 'POST',
			headers: headers(owner),
			body: JSON.stringify({}),
		});
		expect(createInvite.ok).toBeTruthy();
		const invite = (await createInvite.json()) as { code: string };
		const joinServer = await fetch(`${API_URL}/api/v2/servers/join/${invite.code}`, {
			method: 'POST',
			headers: headers(member),
			body: '{}',
		});
		expect(joinServer.ok).toBeTruthy();

		const response = await fetch(`${API_URL}/api/v2/canvas/servers/${server.id}/music/queue`, {
			method: 'POST',
			headers: headers(member),
			body: JSON.stringify({ artist: 'Yapper', title: 'Forbidden', duration_secs: 180 }),
		});

		expect(response.status).toBe(403);
	});
});
