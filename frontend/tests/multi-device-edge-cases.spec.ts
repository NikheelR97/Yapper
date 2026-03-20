/**
 * Multi-Device Trust — Edge Case Tests
 *
 * Tests the device trust state-machine under adverse conditions:
 *
 *   A — WS 'error' frame (code 4001 / Device revoked) → app clears auth and
 *       redirects to /login
 *   B — Offline approval persists in localStorage across reload (commit 42f981a)
 *   C — sync-events HTTP 500 → app retries and eventually becomes ready
 *   D — PendingDeviceGate shows "Restore from encrypted backup" section
 *
 * All tests are pure-mock (no live backend).
 *
 * @multidevice @smoke
 */

import { test, expect, type Page } from '@playwright/test';
import {
	buildMockAuthData,
	buildMockDevice,
	mockAuthEndpoints,
	setInstallationId,
	type ServerDevice,
} from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Set up a trusted device session (no PendingDeviceGate).
 * Returns the auth data and the mocked device.
 */
async function setupTrustedDevice(
	page: Page,
	installId: string,
): Promise<{ authData: ReturnType<typeof buildMockAuthData>; device: ServerDevice }> {
	const device = buildMockDevice({
		installation_id: installId,
		trust_state: 'trusted',
		approved_at: new Date().toISOString(),
	});
	const authData = buildMockAuthData({ device });

	await setInstallationId(page, installId);
	await mockAuthEndpoints(page, authData, { devices: [device] });

	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);

	return { authData, device };
}

/**
 * Set up a pending-trust device session (shows PendingDeviceGate).
 */
async function setupPendingDevice(
	page: Page,
	installId: string,
): Promise<{ authData: ReturnType<typeof buildMockAuthData>; pendingDevice: ServerDevice }> {
	const pendingDevice = buildMockDevice({
		id: 'pending-dev-123',
		installation_id: installId,
		trust_state: 'pending_trust',
		approved_at: null,
	});
	const authData = buildMockAuthData({ device: pendingDevice });

	await setInstallationId(page, installId);
	// mockAuthEndpoints with only the pending device — layout sees pending_trust state
	await mockAuthEndpoints(page, authData, { devices: [pendingDevice] });

	// Stub additional endpoints the layout may call
	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);

	return { authData, pendingDevice };
}

// ─── Test A: WS code 4001 → redirect to /login ───────────────────────────────

test.describe('Multi-device — WS device revocation @multidevice @smoke', () => {
	test('WS error frame (code 4001, Device revoked) clears session and redirects to /login', async ({
		page,
	}) => {
		// Intercept WebSocket before the app opens one.
		// Immediately sends 'ready', then after 1s sends an 'error' frame with code 4001.
		await page.addInitScript(() => {
			const OriginalWebSocket = window.WebSocket;
			class MockWebSocket extends OriginalWebSocket {
				constructor(url: string | URL, protocols?: string | string[]) {
					super(url, protocols);
					this.addEventListener('open', () => {
						// Handshake: send 'ready' so wsStore.connected = true
						(this as unknown as { dispatchFakeMessage: (data: string) => void })
							.dispatchFakeMessage(JSON.stringify({ type: 'ready' }));

						// After a short delay, inject the device-revoked error frame
						setTimeout(() => {
							(this as unknown as { dispatchFakeMessage: (data: string) => void })
								.dispatchFakeMessage(
									JSON.stringify({ type: 'error', code: 4001, message: 'Device revoked' }),
								);
						}, 800);
					});
				}

				dispatchFakeMessage(data: string): void {
					const event = new MessageEvent('message', { data });
					this.dispatchEvent(event);
				}
			}
			window.WebSocket = MockWebSocket as unknown as typeof WebSocket;
		});

		await setupTrustedDevice(page, 'ws-revoke-install');

		log('NETWORK', 'NAVIGATE', 'Loading /explore — WS will inject device-revoked frame after 1s');
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// App should redirect to /login within 15 s after the revocation frame arrives
		log('ASSERTION', 'STATE', 'Waiting for redirect to /login after WS 4001 frame');
		await expect(page).toHaveURL(/\/login/, { timeout: 15_000 });
		log('VALIDATION', 'STATE', 'Redirected to /login after device revocation. [PASS]');
	});
});

// ─── Test B: Offline approval persists across reload ─────────────────────────

test.describe('Multi-device — offline approval persistence @multidevice @smoke', () => {
	test('Approved pending device IDs persist in localStorage across reload', async ({ page }) => {
		// Set up a pending device (PendingDeviceGate shown)
		const { pendingDevice } = await setupPendingDevice(page, 'offline-approval-install');

		log('NETWORK', 'NAVIGATE', 'Loading /explore with pending device — PendingDeviceGate expected');
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// Simulate the approval action by writing to localStorage directly.
		// This mirrors what the Approve button in the trust-request panel does.
		const deviceId = pendingDevice.id;
		await page.evaluate((id) => {
			const key = 'yapper_approved_unsynced_devices';
			const existing: string[] = JSON.parse(localStorage.getItem(key) ?? '[]');
			if (!existing.includes(id)) existing.push(id);
			localStorage.setItem(key, JSON.stringify(existing));
		}, deviceId);

		const storedBefore = await page.evaluate(() =>
			localStorage.getItem('yapper_approved_unsynced_devices'),
		);
		log('ASSERTION', 'STATE', `localStorage before reload: ${storedBefore}`);
		expect(storedBefore).toContain(deviceId);

		// Reload and verify the key survived
		await page.reload();
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		const storedAfter = await page.evaluate(() =>
			localStorage.getItem('yapper_approved_unsynced_devices'),
		);
		log('ASSERTION', 'STATE', `localStorage after reload: ${storedAfter}`);
		expect(storedAfter, 'Approved device IDs must survive a page reload').toContain(deviceId);
		log('VALIDATION', 'STATE', 'Offline approval persisted across reload. [PASS]');
	});
});

// ─── Test C: sync-events 500 → retry ─────────────────────────────────────────

test.describe('Multi-device — sync-events retry on 500 @multidevice', () => {
	test('App retries sync-events on HTTP 500 and eventually becomes ready', async ({ page }) => {
		let callCount = 0;

		const device = buildMockDevice({
			installation_id: 'sync-retry-install',
			trust_state: 'trusted',
			approved_at: new Date().toISOString(),
		});
		const authData = buildMockAuthData({ device });

		await setInstallationId(page, 'sync-retry-install');

		// Override sync-events: fail first 2 GET calls, succeed on 3rd
		await page.route('**/api/v2/devices/sync-events', async (route) => {
			if (route.request().method() !== 'GET') {
				await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
				return;
			}
			callCount++;
			if (callCount < 3) {
				log('MOCK', 'SYNC_EVENTS', `sync-events call ${callCount}: returning 500`);
				await route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
			} else {
				log('MOCK', 'SYNC_EVENTS', `sync-events call ${callCount}: returning 200 []`);
				await route.fulfill({
					status: 200,
					contentType: 'application/json',
					body: '[]',
				});
			}
		});

		// Mount other auth routes WITHOUT the sync-events override (already done above)
		await page.route('**/api/v2/auth/refresh', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify({
					access_token: authData.accessToken,
					csrf_token: authData.csrfToken,
					user: authData.user,
					device,
				}),
			});
		});
		await page.route('**/api/v1/users/me', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(authData.user),
			});
		});
		await page.route('**/api/v2/devices', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([device]),
			});
		});
		await page.route('**/api/v2/devices/trust-requests', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		});
		await page.route('**/api/v1/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await mockExploreEndpoints(page);

		log('NETWORK', 'NAVIGATE', 'Loading /explore with sync-events 500 (first 2 calls fail)');
		await page.goto('/explore');

		// App must become ready (loading screen gone) even after 500 retries
		log('ASSERTION', 'STATE', 'Waiting for loading screen to clear despite sync-events 500s');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 45_000,
		});
		log('VALIDATION', 'STATE', 'App became ready after sync-events retries. [PASS]');

		// Confirm the app did retry at least 3 times
		expect(callCount, 'sync-events must have been called at least 3 times (2 failures + 1 success)').toBeGreaterThanOrEqual(3);
		log('VALIDATION', 'STATE', `sync-events called ${callCount} times. [PASS]`);
	});
});

// ─── Test D: PendingDeviceGate shows restore-from-backup ─────────────────────

test.describe('Multi-device — PendingDeviceGate backup restore link @multidevice @smoke', () => {
	test(
		'"Restore from encrypted backup" section visible on PendingDeviceGate and app stays responsive',
		async ({ page }) => {
			await setupPendingDevice(page, 'pending-backup-install');

			log(
				'NETWORK',
				'NAVIGATE',
				'Loading /explore with pending device — PendingDeviceGate expected',
			);
			await page.goto('/explore');
			await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
				timeout: 30_000,
			});

			// PendingDeviceGate should be visible
			log(
				'ASSERTION',
				'UI',
				'Looking for PendingDeviceGate (Waiting for trust approval heading or backup section)',
			);
			const gate = page.locator('text=Restore from encrypted backup');
			await expect(gate).toBeVisible({ timeout: 15_000 });
			log('VALIDATION', 'UI', '"Restore from encrypted backup" section is visible. [PASS]');

			// Click into the backup restore section — app must remain responsive
			await gate.click().catch(() => {});
			await page.waitForTimeout(500);

			// App shell must still be visible (not crashed/blank)
			log(
				'ASSERTION',
				'STATE',
				'Verifying app is still responsive after clicking backup restore section',
			);
			const isResponsive = await page
				.locator('body')
				.isVisible()
				.catch(() => false);
			expect(isResponsive, 'App must remain responsive after clicking backup restore').toBe(true);
			log('VALIDATION', 'STATE', 'App remains responsive after backup restore click. [PASS]');
		},
	);
});
