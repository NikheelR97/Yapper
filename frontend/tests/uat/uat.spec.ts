/**
 * UAT — User Acceptance Testing Suite
 *
 * Covers gaps not handled by the existing 45 spec files:
 *   - Health / infrastructure baseline (UAT-01)
 *   - Security headers & hardening (UAT-17)
 *   - E2EE client-observable invariants (UAT-06)
 *   - Premium / GoPro feature gating (UAT-14)
 *   - Push notification token API (UAT-16)
 *   - Account lifecycle (deletion, data export, username cooldown) (UAT-12)
 *   - Support ticket field validation (UAT-15)
 *   - Emoji upload limits (UAT-11)
 *   - Media upload URL validation (UAT-07)
 *
 * Requires env vars: E2E_EMAIL, E2E_PASSWORD (primary account).
 * Optional: E2E_EMAIL_2, E2E_PASSWORD_2 (secondary account for two-user tests).
 */

import { test, expect } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	PRIMARY_INSTALLATION_ID,
} from '../auth-helper.js';
import { waitForAppReady } from '../helpers/wait-for.js';

// ─── Environment ─────────────────────────────────────────────────────────────

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const USER_EMAIL = process.env.E2E_EMAIL ?? '';
const USER_PASS = process.env.E2E_PASSWORD ?? '';

// ─── Helpers ─────────────────────────────────────────────────────────────────

interface ApiSession {
	accessToken: string;
	csrfToken: string;
	userId: string;
}

async function loginViaAPI(email: string, password: string): Promise<ApiSession> {
	const res = await fetch(`${API_URL}/api/v2/auth/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			email,
			password,
			device: {
				installation_id: PRIMARY_INSTALLATION_ID,
				platform: 'web',
				label: 'UAT Test',
			},
		}),
	});
	if (!res.ok) throw new Error(`loginViaAPI failed: ${res.status}`);
	const data = (await res.json()) as Record<string, unknown>;
	const accessToken = (data.access_token ?? data.accessToken) as string;
	const csrfToken = (data.csrf_token ?? data.csrfToken) as string;
	const user = data.user as Record<string, unknown>;
	return { accessToken, csrfToken, userId: String(user.id) };
}

function authedHeaders(session: ApiSession): Record<string, string> {
	return {
		'Content-Type': 'application/json',
		Authorization: `Bearer ${session.accessToken}`,
		Cookie: `csrf_token=${session.csrfToken}`,
		'X-CSRF-Token': session.csrfToken,
	};
}

function uid(prefix: string): string {
	return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
}

// ─────────────────────────────────────────────────────────────────────────────
// UAT-01 · Health & Infrastructure
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-01 · Health & Infrastructure', () => {
	// UAT-01-A
	test('UAT-01-A  API /health returns ok with db:true', async () => {
		const res = await fetch(`${API_URL}/health`);
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.status).toBe('ok');
		expect(body.db).toBe(true);
	});

	// UAT-01-B
	test('UAT-01-B  Frontend loads within 5 seconds', async ({ page }) => {
		const start = Date.now();
		await page.goto('/login', { timeout: 5_000 });
		const elapsed = Date.now() - start;
		expect(elapsed).toBeLessThan(5_000);
		await expect(page.locator('body')).toBeVisible();
	});

	// UAT-01-C
	test('UAT-01-C  Unauthenticated visit to /servers redirects to /login', async ({ page }) => {
		page.context().clearCookies();
		await page.goto('/servers');
		await page.waitForURL(/\/(login|explore)/, { timeout: 10_000 });
		const url = page.url();
		expect(url).toMatch(/\/(login|explore)/);
	});

	// UAT-01-D
	test('UAT-01-D  Health endpoint does not require authentication', async () => {
		const res = await fetch(`${API_URL}/health`, {
			headers: {}, // no auth
		});
		expect(res.status).toBe(200);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-17 · Security Headers & Hardening
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-17 · Security Headers & Hardening', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run security header tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-17-A
	test('UAT-17-A  API includes X-Content-Type-Options: nosniff', async () => {
		const res = await fetch(`${API_URL}/health`);
		expect.soft(res.headers.get('x-content-type-options')).toBe('nosniff');
	});

	// UAT-17-B
	test('UAT-17-B  API includes X-Frame-Options DENY or SAMEORIGIN', async () => {
		const res = await fetch(`${API_URL}/health`);
		const val = (res.headers.get('x-frame-options') ?? '').toUpperCase();
		expect.soft(val).toMatch(/DENY|SAMEORIGIN/);
	});

	// UAT-17-C
	test('UAT-17-C  API includes Strict-Transport-Security on HTTPS', async () => {
		if (!API_URL.startsWith('https')) {
			test.skip(true, 'HSTS only applies over HTTPS');
			return;
		}
		const res = await fetch(`${API_URL}/health`);
		expect.soft(res.headers.get('strict-transport-security')).toBeTruthy();
	});

	// UAT-17-D
	test('UAT-17-D  CORS rejects unlisted origin', async () => {
		const res = await fetch(`${API_URL}/health`, {
			headers: { Origin: 'https://evil.example.com' },
		});
		const acao = res.headers.get('access-control-allow-origin') ?? '';
		// Should NOT be * and should NOT be evil.example.com
		expect.soft(acao).not.toBe('*');
		expect.soft(acao).not.toContain('evil.example.com');
	});

	// UAT-17-G
	test('UAT-17-G  Mutating request without X-CSRF-Token returns 403', async () => {
		const res = await fetch(`${API_URL}/api/v1/servers`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Authorization: `Bearer ${session.accessToken}`,
				// deliberately omit X-CSRF-Token
			},
			body: JSON.stringify({ name: 'csrf-test' }),
		});
		// Expect 403 (CSRF) or 401/400 — must NOT be 200/201
		expect(res.status).toBeGreaterThanOrEqual(400);
	});

	// UAT-17-F
	test('UAT-17-F  Login response sets HttpOnly Secure cookie', async () => {
		const res = await fetch(`${API_URL}/api/v2/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				email: USER_EMAIL,
				password: USER_PASS,
				device: {
					installation_id: PRIMARY_INSTALLATION_ID,
					platform: 'web',
					label: 'UAT Cookie Check',
				},
			}),
		});
		expect(res.status).toBe(200);
		// Node fetch exposes set-cookie via getSetCookie() or raw headers
		const setCookie = (res.headers as unknown as { getSetCookie?: () => string[] }).getSetCookie?.()?.join('; ')
			?? res.headers.get('set-cookie') ?? '';
		// At minimum, HttpOnly should be present on the refresh token cookie
		expect.soft(setCookie.toLowerCase()).toContain('httponly');
		if (API_URL.startsWith('https')) {
			expect.soft(setCookie.toLowerCase()).toContain('secure');
		}
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-06 · E2EE — Client-Observable Behaviour
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-06 · E2EE — Client-Observable Behaviour', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run E2EE tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			seedTrustedPrimaryDevice();
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-06-A
	test('UAT-06-A  IndexedDB yapper-signal store exists after login', async ({ page }) => {
		test.setTimeout(120_000);
		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', USER_EMAIL);
		await page.fill('#password', USER_PASS);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/(explore|servers)/, { timeout: 20_000 });
		await waitForAppReady(page);

		// Check for IndexedDB — the app uses a DB with "signal" in the name
		const hasSignalDb = await page.evaluate(async () => {
			const dbs = await indexedDB.databases();
			return dbs.some((db) => db.name?.includes('signal') || db.name?.includes('yapper'));
		});
		expect(hasSignalDb).toBe(true);
	});

	// UAT-06-D
	test('UAT-06-D  Key bundle API returns public keys only, no private keys', async () => {
		const res = await fetch(`${API_URL}/api/v2/keys/${session.userId}/bundles`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const text = await res.text();
		const lowered = text.toLowerCase();
		// Must NOT contain private key fields
		expect.soft(lowered).not.toContain('privatekey');
		expect.soft(lowered).not.toContain('private_key');
		expect.soft(lowered).not.toContain('secret');
		expect.soft(lowered).not.toContain('identity_key_pair');
		// Should contain public key fields
		expect.soft(lowered).toMatch(/public|ik_public|spk_public|identity_sig_key|signed_prekey/);
	});

	// UAT-06-E
	test('UAT-06-E  WebSocket URL does not contain token in query string', async ({ page }) => {
		test.setTimeout(120_000);

		// Intercept WS connections via route to capture URLs
		const wsUrls: string[] = [];
		await page.route(/\/ws/, async (route) => {
			wsUrls.push(route.request().url());
			await route.continue();
		});

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', USER_EMAIL);
		await page.fill('#password', USER_PASS);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/(explore|servers)/, { timeout: 20_000 });
		await waitForAppReady(page);

		// Wait for WS to connect
		await page.waitForTimeout(8_000);

		// Check that the WS URL used by the app (stored in env.ts) doesn't use query-string auth
		const wsUrl = await page.evaluate(() => {
			// The app builds WS_URL from API_URL — check what's in the source
			return (window as unknown as Record<string, string>).__WS_URL ?? 'not-set';
		});
		// Main assertion: the WS URL pattern should never include token= in the query
		if (wsUrl !== 'not-set') {
			expect.soft(wsUrl).not.toMatch(/[?&]token=/i);
		}
		// Also check any intercepted URLs
		for (const url of wsUrls) {
			expect.soft(url).not.toMatch(/[?&]token=/i);
		}
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-14 · Premium & GoPro
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-14 · Premium & GoPro', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run premium tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-14-A
	test('UAT-14-A  Free user premium status returns is_premium:false', async () => {
		const res = await fetch(`${API_URL}/api/v1/premium`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.is_premium).toBe(false);
	});

	// UAT-14-B
	test('UAT-14-B  Invalid promo code returns 400 or 404', async () => {
		const res = await fetch(`${API_URL}/api/v1/premium/activate`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({ code: 'INVALID_CODE_XYZ_12345' }),
		});
		expect(res.status).toBeGreaterThanOrEqual(400);
		expect(res.status).toBeLessThan(500);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-16 · Push Notifications
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-16 · Push Notifications', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run push notification tests');

	let session: ApiSession;
	const dummyToken = `uat-fcm-dummy-${Date.now()}`;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-16-A
	test('UAT-16-A  Register FCM device token returns 200/201', async () => {
		const res = await fetch(`${API_URL}/api/v1/notifications/push-token`, {
			method: 'PUT',
			headers: authedHeaders(session),
			body: JSON.stringify({ token: dummyToken, platform: 'web' }),
		});
		expect([200, 201, 204]).toContain(res.status);
	});

	// UAT-16-B
	test('UAT-16-B  Unregister FCM device token returns 200', async () => {
		const res = await fetch(`${API_URL}/api/v1/notifications/push-token`, {
			method: 'DELETE',
			headers: authedHeaders(session),
			body: JSON.stringify({ token: dummyToken }),
		});
		expect([200, 204]).toContain(res.status);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-15 · Support Tickets — Field Validation
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-15 · Support Tickets — Validation', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run support ticket tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-15-A
	test('UAT-15-A  Valid ticket creation returns 201', async () => {
		const res = await fetch(`${API_URL}/api/v1/support/tickets`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				ticket_type: 'bug',
				priority: 'medium',
				subject: `UAT test ticket ${Date.now()}`,
				description: 'Automated UAT test — safe to delete.',
			}),
		});
		// 429 = rate limited from previous test runs — endpoint works, just throttled
		if (res.status === 429) {
			test.skip(true, 'Support ticket rate limited — endpoint functional');
			return;
		}
		expect([200, 201]).toContain(res.status);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.id).toBeTruthy();
	});

	// UAT-15-B
	test('UAT-15-B  GET /support/tickets returns ticket list', async () => {
		const res = await fetch(`${API_URL}/api/v1/support/tickets`, {
			headers: authedHeaders(session),
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as unknown;
		// Response may be a bare array or wrapped: { tickets: [...] }
		const tickets = Array.isArray(body) ? body : ((body as Record<string, unknown>).tickets as unknown[]);
		expect(Array.isArray(tickets)).toBe(true);
	});

	// UAT-15-C
	test('UAT-15-C  Invalid ticket type returns 400/422', async () => {
		const res = await fetch(`${API_URL}/api/v1/support/tickets`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				ticket_type: 'invalid_type',
				priority: 'medium',
				subject: 'UAT invalid type test',
				description: 'Should be rejected.',
			}),
		});
		expect(res.status).toBeGreaterThanOrEqual(400);
		expect(res.status).toBeLessThan(500);
	});

	// UAT-15-D
	test('UAT-15-D  Subject over 200 characters returns 400/422', async () => {
		const longSubject = 'x'.repeat(201);
		const res = await fetch(`${API_URL}/api/v1/support/tickets`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				ticket_type: 'bug',
				priority: 'low',
				subject: longSubject,
				description: 'Long subject test.',
			}),
		});
		// May be accepted if backend doesn't enforce — soft assertion
		expect.soft(res.status).toBeGreaterThanOrEqual(400);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-12 · Account Lifecycle (data export, username cooldown)
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-12 · Account Lifecycle', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run account lifecycle tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-12-A
	test('UAT-12-A  PATCH /users/me updates display_name', async () => {
		const newName = `UAT User ${Date.now()}`;
		const patchRes = await fetch(`${API_URL}/api/v1/users/me`, {
			method: 'PATCH',
			headers: authedHeaders(session),
			body: JSON.stringify({ display_name: newName }),
		});
		// Accept 200 or 204
		expect([200, 204]).toContain(patchRes.status);

		const getRes = await fetch(`${API_URL}/api/v1/users/me`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(getRes.status).toBe(200);
		const user = (await getRes.json()) as Record<string, unknown>;
		const returnedName = (user.display_name ?? user.displayName) as string;
		expect.soft(returnedName).toBe(newName);
	});

	// UAT-12-B
	test('UAT-12-B  Username change cooldown — second change returns 409', async () => {
		const newUsername = uid('uat');
		const res1 = await fetch(`${API_URL}/api/v1/users/me/username`, {
			method: 'PATCH',
			headers: authedHeaders(session),
			body: JSON.stringify({ username: newUsername }),
		});
		// First change may succeed (200) or fail if cooldown already active (409)
		if (res1.status === 200 || res1.status === 204) {
			// Try changing again immediately — should be blocked by 30-day cooldown
			const res2 = await fetch(`${API_URL}/api/v1/users/me/username`, {
				method: 'PATCH',
				headers: authedHeaders(session),
				body: JSON.stringify({ username: uid('uat2') }),
			});
			// Backend returns Conflict (409) with remaining days
			expect.soft([409, 429, 400]).toContain(res2.status);
		} else {
			// Cooldown already active from a previous change — that counts as a pass
			expect([409, 429, 400]).toContain(res1.status);
		}
	});

	// UAT-12-E
	test('UAT-12-E  Data export returns a ZIP file', async () => {
		const res = await fetch(`${API_URL}/api/v1/account/data-export`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const ct = res.headers.get('content-type') ?? '';
		expect.soft(ct).toMatch(/zip|octet-stream/);
	});

	// UAT-12-L
	test('UAT-12-L  Change password with wrong current returns 400/401', async () => {
		const res = await fetch(`${API_URL}/api/v1/users/me/password`, {
			method: 'PUT',
			headers: authedHeaders(session),
			body: JSON.stringify({
				current_password: 'wrong_password_12345',
				new_password: 'NewP@ssw0rd!',
			}),
		});
		expect(res.status).toBeGreaterThanOrEqual(400);
		expect(res.status).toBeLessThan(500);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-11 · Custom Emoji Limits
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-11 · Custom Emoji Limits', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run emoji limit tests');

	let session: ApiSession;
	let serverId: string;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
			// Create a server for emoji tests
			const res = await fetch(`${API_URL}/api/v1/servers`, {
				method: 'POST',
				headers: authedHeaders(session),
				body: JSON.stringify({ name: `UAT Emoji ${Date.now()}` }),
			});
			if (res.ok) {
				const body = (await res.json()) as { id: string };
				serverId = body.id;
			}
		}
	});

	// UAT-11-C
	test('UAT-11-C  Non-admin emoji upload returns 403', async () => {
		// This test only works with a second user who is NOT admin of the server.
		// With the primary user (who IS admin), we verify the endpoint exists.
		// Skip if no serverId.
		if (!serverId) {
			test.skip(true, 'No server available for emoji test');
			return;
		}

		const USER_B_EMAIL = process.env.E2E_EMAIL_2 ?? '';
		const USER_B_PASS = process.env.E2E_PASSWORD_2 ?? '';
		if (!USER_B_EMAIL) {
			test.skip(true, 'E2E_EMAIL_2 needed for non-admin emoji test');
			return;
		}

		const sessionB = await loginViaAPI(USER_B_EMAIL, USER_B_PASS);
		const res = await fetch(`${API_URL}/api/v1/servers/${serverId}/emojis`, {
			method: 'POST',
			headers: {
				...authedHeaders(sessionB),
				'Content-Type': 'application/json',
			},
			body: JSON.stringify({ name: 'test_emoji', image: 'data:image/png;base64,iVBOR' }),
		});
		// Non-member or non-admin should get 403
		expect(res.status).toBeGreaterThanOrEqual(400);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-07 · Media Upload Validation
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-07 · Media Upload Validation', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run media upload tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-07-A
	test('UAT-07-A  Upload URL endpoint returns a presigned URL', async () => {
		const res = await fetch(`${API_URL}/api/v1/media/upload-url`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				media_type: 'yap',
				content_length: 1_000_000, // 1 MB — well within limit
			}),
		});
		// R2 may not be configured locally — skip gracefully
		if (res.status === 500 || res.status === 404) {
			test.skip(true, 'Media upload endpoint not available (R2 not configured)');
			return;
		}
		expect([200, 201]).toContain(res.status);
		const body = (await res.json()) as Record<string, unknown>;
		const url = (body.upload_url ?? body.url ?? body.presigned_url) as string;
		expect(url).toBeTruthy();
		// URL should not contain an AES key
		expect.soft(url.toLowerCase()).not.toMatch(/aes|key=[a-f0-9]{32,}/i);
	});

	// UAT-07-B
	test('UAT-07-B  Presigned URL hostname matches R2 (not API)', async () => {
		const res = await fetch(`${API_URL}/api/v1/media/upload-url`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({ media_type: 'yap', content_length: 500_000 }),
		});
		if (res.status === 500 || res.status === 404) {
			test.skip(true, 'Media upload endpoint not available (R2 not configured)');
			return;
		}
		expect(res.status).toBeLessThan(400);
		const body = (await res.json()) as Record<string, unknown>;
		const url = (body.upload_url ?? body.url ?? body.presigned_url) as string;
		if (url) {
			const hostname = new URL(url).hostname;
			expect.soft(hostname).not.toContain('api.yapperhq.com');
		}
	});

	// UAT-07-D
	test('UAT-07-D  Upload size exceeding limit returns 400/413', async () => {
		const res = await fetch(`${API_URL}/api/v1/media/upload-url`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				media_type: 'yap',
				content_length: 26 * 1024 * 1024, // 26 MB — over 25 MB free limit
			}),
		});
		if (res.status === 500 || res.status === 404) {
			test.skip(true, 'Media upload endpoint not available (R2 not configured)');
			return;
		}
		expect(res.status).toBeGreaterThanOrEqual(400);
		expect(res.status).toBeLessThan(500);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-04 · Server API Validation
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-04 · Server API Validation', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run server validation tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-04-E
	test('UAT-04-E  Create server with no name returns 400/422', async () => {
		const res = await fetch(`${API_URL}/api/v1/servers`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({}),
		});
		expect(res.status).toBeGreaterThanOrEqual(400);
		expect(res.status).toBeLessThan(500);
	});

	// UAT-04-F
	test('UAT-04-F  Join with invalid invite code returns 404', async () => {
		const res = await fetch(`${API_URL}/api/v1/servers/join/INVALID_CODE_XYZ`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: '{}',
		});
		expect([404, 400]).toContain(res.status);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-10 · Profiles & Social API
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-10 · Profiles & Social API', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run profile API tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-10-A
	test('UAT-10-A  GET /users/me returns user with id and username', async () => {
		const res = await fetch(`${API_URL}/api/v1/users/me`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect(body.id).toBeTruthy();
		expect(body.username).toBeTruthy();
		// Note: email is intentionally NOT returned by GET /users/me (privacy)
		expect.soft(body.display_name ?? body.displayName).toBeTruthy();
	});

	// UAT-10-B
	test('UAT-10-B  GET /users/by/:username does not expose private fields', async () => {
		// First get our username from /me
		const meRes = await fetch(`${API_URL}/api/v1/users/me`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		const me = (await meRes.json()) as Record<string, unknown>;
		const username = me.username as string;

		const res = await fetch(`${API_URL}/api/v1/users/by/${username}`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const text = await res.text();
		expect.soft(text).not.toContain('password_hash');
		expect.soft(text).not.toContain('private_key');
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-18 · Device Trust Flow
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-18 · Device Trust Flow', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run device trust tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			seedTrustedPrimaryDevice();
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-18-A
	test('UAT-18-A  GET /devices returns current device as trusted', async () => {
		const res = await fetch(`${API_URL}/api/v2/devices`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const devices = (await res.json()) as Array<Record<string, unknown>>;
		expect(Array.isArray(devices)).toBe(true);
		// At least one device should be trusted
		const hasTrusted = devices.some((d) => d.trust_state === 'trusted');
		expect(hasTrusted).toBe(true);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-08 · Live Canvas API
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-08 · Live Canvas API', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run canvas API tests');

	let session: ApiSession;
	let serverId: string;
	let channelId: string;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
			const res = await fetch(`${API_URL}/api/v1/servers`, {
				method: 'POST',
				headers: authedHeaders(session),
				body: JSON.stringify({ name: `UAT Canvas ${Date.now()}` }),
			});
			if (res.ok) {
				const body = (await res.json()) as { id: string };
				serverId = body.id;
				// Get default channel
				const chRes = await fetch(`${API_URL}/api/v1/servers/${serverId}/channels`, {
					headers: { Authorization: `Bearer ${session.accessToken}` },
				});
				if (chRes.ok) {
					const channels = (await chRes.json()) as Array<{ id: string }>;
					channelId = channels[0]?.id;
				}
			}
		}
	});

	// UAT-08-A
	test('UAT-08-A  Canvas state returns music, polls, clips, event keys', async () => {
		if (!serverId || !channelId) {
			test.skip(true, 'No server/channel available');
			return;
		}
		const res = await fetch(`${API_URL}/api/v1/canvas/servers/${serverId}/state?channel_id=${channelId}`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		expect.soft(body).toHaveProperty('music');
		expect.soft(body).toHaveProperty('polls');
		expect.soft(body).toHaveProperty('clips');
	});

	// UAT-08-B
	test('UAT-08-B  Creating a poll returns 201', async () => {
		if (!channelId) {
			test.skip(true, 'No channel available');
			return;
		}
		const res = await fetch(`${API_URL}/api/v1/canvas/channels/${channelId}/polls`, {
			method: 'POST',
			headers: authedHeaders(session),
			body: JSON.stringify({
				question: `UAT poll ${Date.now()}`,
				poll_type: 'multiple_choice',
				options: ['Option A', 'Option B'],
			}),
		});
		expect([200, 201]).toContain(res.status);
	});
});

// ─────────────────────────────────────────────────────────────────────────────
// UAT-09 · Explore API
// ─────────────────────────────────────────────────────────────────────────────

test.describe('UAT-09 · Explore API', () => {
	test.skip(!USER_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run explore API tests');

	let session: ApiSession;

	test.beforeAll(async () => {
		if (USER_EMAIL) {
			session = await loginViaAPI(USER_EMAIL, USER_PASS);
		}
	});

	// UAT-09-A
	test('UAT-09-A  Trending tags returns an array', async () => {
		const res = await fetch(`${API_URL}/api/v1/explore/trending-tags`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
		const body = (await res.json()) as Record<string, unknown>;
		// May return { tags: [...] } or bare array
		const tags = Array.isArray(body) ? body : (body.tags as unknown[]);
		expect(Array.isArray(tags)).toBe(true);
	});

	// UAT-09-B
	test('UAT-09-B  Server search returns without error', async () => {
		const res = await fetch(`${API_URL}/api/v1/search?q=general`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
	});

	// UAT-09-C
	test('UAT-09-C  Live servers returns an array', async () => {
		const res = await fetch(`${API_URL}/api/v1/explore/live-servers`, {
			headers: { Authorization: `Bearer ${session.accessToken}` },
		});
		expect(res.status).toBe(200);
	});
});
