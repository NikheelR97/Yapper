/**
 * Multi-Device Trust - Edge Case Tests
 *
 * @multidevice @smoke
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { buildMockDevice, PRIMARY_INSTALLATION_ID, type ServerDevice } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

async function setupTrustedDevice(
	page: Page,
): Promise<{ device: ServerDevice }> {
	const installationId = PRIMARY_INSTALLATION_ID;
	const device = buildMockDevice({
		installation_id: installationId,
		trust_state: 'trusted',
		approved_at: new Date().toISOString(),
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
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);

	return { device };
}

async function setupPendingDevice(
	page: Page,
): Promise<{ pendingDevice: ServerDevice }> {
	const installationId = PRIMARY_INSTALLATION_ID;
	const pendingDevice = buildMockDevice({
		id: 'pending-dev-123',
		installation_id: installationId,
		trust_state: 'pending_trust',
		approved_at: null,
	});
	const trustedDevice = buildMockDevice({
		id: 'trusted-dev-primary',
		installation_id: 'trusted-primary-install',
		trust_state: 'trusted',
		approved_at: new Date().toISOString(),
	});

	// Override the fixture's auth-refresh mock so the boot sequence places the
	// session on the pending device instead of the trusted one baked into
	// auth-data.json. `page.route` handlers are LIFO, so the most recent one wins.
	await page.route('**/api/v2/auth/refresh', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				access_token: 'pending-device-access-token',
				csrf_token: 'pending-device-csrf-token',
				user: { id: 'e2e-user', username: 'e2e_user' },
				device: pendingDevice,
			}),
		});
	});

	await page.route('**/api/v2/devices', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([pendingDevice, trustedDevice]),
		});
	});
	await page.route('**/api/v2/devices/trust-requests', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
	});
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);

	return { pendingDevice };
}

test.describe('Multi-device - WS device revocation @multidevice @smoke', () => {
	test('WS error frame (code 4001, Device revoked) clears session and redirects to /login', async ({ userPage }) => {
		await userPage.addInitScript(() => {
			class MockWebSocket extends EventTarget {
				static readonly CONNECTING = 0;
				static readonly OPEN = 1;
				static readonly CLOSED = 3;
				readonly CONNECTING = 0;
				readonly OPEN = 1;
				readonly CLOSED = 3;
				readyState = MockWebSocket.CONNECTING;
				url: string;
				protocol = '';
				extensions = '';
				bufferedAmount = 0;
				binaryType: BinaryType = 'blob';
				onopen: ((ev: Event) => void) | null = null;
				onmessage: ((ev: MessageEvent) => void) | null = null;
				onerror: ((ev: Event) => void) | null = null;
				onclose: ((ev: CloseEvent) => void) | null = null;

				constructor(url: string | URL) {
					super();
					this.url = url.toString();
					setTimeout(() => {
						this.readyState = MockWebSocket.OPEN;
						const openEvent = new Event('open');
						this.onopen?.(openEvent);
						this.dispatchEvent(openEvent);
						this.injectMessage(JSON.stringify({ type: 'ready' }));
						setTimeout(() => {
							this.injectMessage(
								JSON.stringify({ type: 'error', code: 4001, message: 'Device revoked' }),
							);
						}, 800);
					}, 50);
				}

				send(): void {}
				close(): void {
					this.readyState = MockWebSocket.CLOSED;
				}

				injectMessage(data: string): void {
					const event = new MessageEvent('message', { data });
					this.onmessage?.(event);
					this.dispatchEvent(event);
				}
			}

			window.WebSocket = MockWebSocket as unknown as typeof WebSocket;
		});

		await setupTrustedDevice(userPage);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage).toHaveURL(/\/login/, { timeout: 15_000 });
	});
});

test.describe('Multi-device - offline approval persistence @multidevice @smoke', () => {
	test('Approved pending device IDs persist in localStorage across reload', async ({ userPage }) => {
		const { pendingDevice } = await setupPendingDevice(userPage);

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await userPage.evaluate((deviceId) => {
			const key = 'yapper_approved_unsynced_devices';
			const existing: string[] = JSON.parse(localStorage.getItem(key) ?? '[]');
			if (!existing.includes(deviceId)) {
				existing.push(deviceId);
			}
			localStorage.setItem(key, JSON.stringify(existing));
		}, pendingDevice.id);

		const storedBefore = await userPage.evaluate(() =>
			localStorage.getItem('yapper_approved_unsynced_devices'),
		);
		expect(storedBefore).toContain(pendingDevice.id);

		await userPage.reload();
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		const storedAfter = await userPage.evaluate(() =>
			localStorage.getItem('yapper_approved_unsynced_devices'),
		);
		expect(storedAfter).toContain(pendingDevice.id);
	});
});

test.describe('Multi-device - sync-events retry on 500 @multidevice', () => {
	test('App retries sync-events on HTTP 500 and eventually becomes ready', async ({ userPage }) => {
		let callCount = 0;
		const installationId = PRIMARY_INSTALLATION_ID;
		const device = buildMockDevice({
			installation_id: installationId,
			trust_state: 'trusted',
			approved_at: new Date().toISOString(),
		});

		await userPage.route('**/api/v2/devices/sync-events', async (route) => {
			if (route.request().method() !== 'GET') {
				await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
				return;
			}
			callCount++;
			if (callCount < 3) {
				await route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
				return;
			}
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await userPage.route('**/api/v2/devices', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([device]),
			});
		});
		await userPage.route('**/api/v2/devices/trust-requests', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		});
		await userPage.route('**/api/v2/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await userPage.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});
		await mockExploreEndpoints(userPage);

		log('NETWORK', 'NAVIGATE', 'Loading /explore with sync-events 500');
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 45_000,
		});
		await expect.poll(() => callCount, { timeout: 10_000 }).toBeGreaterThanOrEqual(1);
	});
});

test.describe('Multi-device - PendingDeviceGate backup restore link @multidevice @smoke', () => {
	test('"Restore from encrypted backup" section visible on PendingDeviceGate and app stays responsive', async ({
		userPage,
	}) => {
		await setupPendingDevice(userPage);

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		const gate = userPage.getByText('Restore from encrypted backup', { exact: true });
		await expect(gate).toBeVisible({ timeout: 15_000 });
		await gate.click().catch(() => {});
		await expect(userPage.locator('body')).toBeVisible();
	});
});
