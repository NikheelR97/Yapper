/**
 * Codex Security Report - Section 3.3
 * Feature: E2E Encryption Flow Under Extreme Network Turbulence
 *
 * @e2ee @websocket @deep-analytical
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await mockExploreEndpoints(page);
}

test.describe('WebSocket MITM disruption @e2ee @websocket @deep-analytical', () => {
	test('forceful WS termination mid-session triggers reconnecting banner', async ({ userPage }) => {
		await setupShellData(userPage);

		log(
			'NETWORK',
			'WEBSOCKET',
			'Installing WebSocket route interceptor - will simulate MITM drop after handshake',
		);

		await userPage.routeWebSocket('**', (ws) => {
			log('NETWORK', 'WEBSOCKET', `WebSocket connection intercepted: ${ws.url()}`);

			let handshakeDone = false;
			ws.onMessage(() => {
				if (!handshakeDone) {
					handshakeDone = true;
					ws.send(JSON.stringify({ type: 'ready' }));
					setTimeout(() => {
						ws.close({ code: 1011, reason: 'MITM disruption' });
					}, 200);
				}
			});
		});

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('.reconnecting-banner')).toBeVisible({ timeout: 15_000 });
	});
});
