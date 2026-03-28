import { test, expect } from '@playwright/test';
import { loginViaApi } from './auth-helper.js';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

type Session = Awaited<ReturnType<typeof loginViaApi>>;

function device(label: string) {
	return {
		installation_id: `${label}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
		platform: 'web',
		label,
	};
}

function authedHeaders(session: Session) {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

async function registerAdult(prefix: string) {
	const email = `${prefix}-${Date.now()}@integration.test`;
	const password = `AdultPass123!-${prefix}`;
	const response = await fetch(`${API_URL}/api/v2/auth/register`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			email,
			username: prefix.replace(/[^a-z0-9_]/gi, '_'),
			password,
			display_name: prefix,
			date_of_birth: '1990-03-27',
			device: device(`${prefix}-register`),
		}),
	});
	expect(response.ok).toBeTruthy();
	return { email, password };
}

async function createParentManagedChild(parent: Session, prefix: string) {
	const email = `${prefix}-${Date.now()}@integration.test`;
	const password = `ChildPass123!-${prefix}`;
	const username = `${prefix}_${Date.now()}`.replace(/[^a-z0-9_]/gi, '_');
	const response = await fetch(`${API_URL}/api/v2/parental/children`, {
		method: 'POST',
		headers: authedHeaders(parent),
		body: JSON.stringify({
			username,
			display_name: prefix,
			email,
			password,
			date_of_birth: '2015-03-26',
		}),
	});
	expect(response.ok).toBeTruthy();
	const body = (await response.json()) as { child_id: string };
	return { email, password, username, userId: body.child_id };
}

test.describe('Parental approval gate @security @e2ee @coppa', () => {
	test('child DM creation is blocked for non-friends and pending approval does not unlock key bundles', async () => {
		const parentCreds = await registerAdult('coppa_parent');
		const strangerCreds = await registerAdult('coppa_stranger');
		const parent = await loginViaApi(parentCreds.email, parentCreds.password, {
			installationId: device('coppa-parent-login').installation_id,
			label: 'COPPA Parent',
		});
		const child = await createParentManagedChild(parent, 'coppa_child');
		const childSession = await loginViaApi(child.email, child.password, {
			installationId: device('coppa-child-login').installation_id,
			label: 'COPPA Child',
		});
		const stranger = await loginViaApi(strangerCreds.email, strangerCreds.password, {
			installationId: device('coppa-stranger-login').installation_id,
			label: 'COPPA Stranger',
		});
		const strangerUserId = String(stranger.user.id);

		const dmAttempt = await fetch(`${API_URL}/api/v2/conversations`, {
			method: 'POST',
			headers: authedHeaders(childSession),
			body: JSON.stringify({ peer_id: strangerUserId }),
		});
		expect(dmAttempt.status).toBe(403);

		const friendRequest = await fetch(
			`${API_URL}/api/v2/users/by/${child.username}/friend-request`,
			{ method: 'POST', headers: authedHeaders(stranger), body: '{}' },
		);
		expect(friendRequest.status).toBe(202);

		const bundleResponse = await fetch(`${API_URL}/api/v2/keys/${child.userId}/bundles`, {
			headers: { Authorization: `Bearer ${stranger.accessToken}` },
		});
		expect(bundleResponse.status).toBe(403);
	});
});
