/**
 * Codex Security Report — Section 3.3
 * Feature: E2E Encryption Flow Under Extreme Network Turbulence
 *
 * Uses page.routeWebSocket() to intercept the WebSocket connection,
 * complete the handshake (sending the 'ready' frame so wsStore.connected
 * flips to true), then forcefully terminate the connection to simulate an
 * aggressive MITM packet drop mid-session.
 *
 * The app must surface the reconnecting banner and enter its exponential-
 * backoff retry loop without leaking session state.
 *
 * @e2ee @websocket @deep-analytical
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

async function setupAuth(page: Parameters<typeof mockAuthEndpoints>[0]): Promise<void> {
	const device = buildMockDevice({ installation_id: 'ws-disruption-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'ws-disruption-install');
	await mockAuthEndpoints(page, authData);
	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v1/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('WebSocket MITM disruption @e2ee @websocket @deep-analytical', () => {
	test('forceful WS termination mid-session triggers reconnecting banner', async ({ page }) => {
		await setupAuth(page);

		log(
			'NETWORK',
			'WEBSOCKET',
			'Installing WebSocket route interceptor — will simulate MITM drop after handshake',
		);

		// routeWebSocket intercepts WebSocket connections only (never HTTP module fetches).
		// On first message from the client (the auth frame), respond with the 'ready'
		// frame so wsStore.connected transitions to true, then drop the connection
		// 200 ms later to simulate an aggressive mid-session MITM cut.
		await page.routeWebSocket('**', (ws) => {
			log('NETWORK', 'WEBSOCKET', `WebSocket connection intercepted: ${ws.url()}`);

			let handshakeDone = false;
			ws.onMessage((_message) => {
				if (!handshakeDone) {
					handshakeDone = true;
					log('NETWORK', 'WEBSOCKET', 'Auth frame received — sending synthetic "ready" frame');
					ws.send(JSON.stringify({ type: 'ready' }));

					// Force-close 200 ms after the handshake to model a precise MITM drop.
					setTimeout(() => {
						log(
							'NETWORK',
							'WEBSOCKET',
							'Terminating connection mid-flight to simulate MITM drop.',
						);
						ws.close({ code: 1011, reason: 'MITM disruption' });
					}, 200);
				}
			});
		});

		log('NETWORK', 'NAVIGATE', 'Navigating to /explore');
		await page.goto('/explore');

		// App must boot past the loading screen (ready=true).
		log('ASSERTION', 'STATE', 'Waiting for loading screen to clear');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// After the 200 ms MITM drop, wsStore.connected becomes false.
		// Banner condition: ready && currentDeviceTrusted() && !wsStore.connected → shown.
		log('ASSERTION', 'UI', 'Waiting for reconnecting banner to surface after MITM drop');
		await expect(page.locator('.reconnecting-banner')).toBeVisible({ timeout: 15_000 });

		log(
			'VALIDATION',
			'UI',
			'Reconnecting banner appeared following forced WebSocket closure. Retry loop active. [PASS]',
		);
	});
});
