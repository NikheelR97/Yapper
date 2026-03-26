/**
 * Codex Security Report — Section 3.4 (Extended)
 * Feature: XSS / Injection Neutralization Across All Input Surfaces
 *
 * Verifies that Svelte's auto-escaping neutralises malicious payloads across
 * every surface where server-supplied text is rendered into the DOM:
 *
 *   1. Explore search results — user display names
 *   2. DM conversation list — peer display names
 *   3. Server sidebar — server names
 *   4. Canvas panel — poll question text
 *   5. Settings / Support — ticket description text
 *
 * Each test installs:
 *   • A tripwire route on xss-canary.internal (any fetch → fail)
 *   • Mocked auth + feature-specific API responses containing XSS payloads
 *
 * Assertions (same 5 per surface):
 *   A1 — No <img onerror> injected into live DOM
 *   A2 — No <svg onload> injected into live DOM
 *   A3 — window.__xss_fired is falsy (no script/svg/onerror execution)
 *   A4 — No exfiltration request to xss-canary.internal
 *   A5 — No XSS-related console errors
 *
 * @security @xss @smoke
 */

import { test, expect, type Page } from '@playwright/test';
import {
	buildMockAuthData,
	buildMockDevice,
	mockAuthEndpoints,
	setInstallationId,
} from './auth-helper.js';
import {
	mockExploreEndpoints,
	mockCanvasEndpoints,
	mockSupportEndpoints,
} from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

// ─── Payload suite ────────────────────────────────────────────────────────────

/** Core XSS vectors that exercise all three execution channels. */
const XSS_PAYLOADS = [
	'<img src=x onerror="window.__xss_fired=true">',
	'<script>window.__xss_fired=true</script>',
	'<svg onload="window.__xss_fired=true">',
];

const XSS_PAYLOAD_PRIMARY = XSS_PAYLOADS[0]; // used for single-value fields

// ─── Shared helpers ───────────────────────────────────────────────────────────

/**
 * Install a tripwire on xss-canary.internal and a console-error collector.
 * Returns { getXssFetchMade, getConsoleErrors }.
 */
function installTripwires(page: Page): {
	getXssFetchMade: () => boolean;
	getConsoleErrors: () => string[];
} {
	let xssFetchMade = false;
	const consoleErrors: string[] = [];

	page.route('**xss-canary.internal**', async (route) => {
		xssFetchMade = true;
		log('SECURITY', 'XSS_TRIPWIRE', `ALERT — exfiltration request: ${route.request().url()}`);
		await route.abort();
	});

	page.on('console', (msg) => {
		if (msg.type() === 'error') consoleErrors.push(msg.text());
	});

	return {
		getXssFetchMade: () => xssFetchMade,
		getConsoleErrors: () => consoleErrors,
	};
}

/**
 * Run the standard 5-assertion XSS gauntlet.
 * Shared across all surfaces to keep assertions consistent.
 */
async function assertXssNeutralized(
	page: Page,
	surface: string,
	getXssFetchMade: () => boolean,
	getConsoleErrors: () => string[],
): Promise<void> {
	// Allow any async onerror / onload handlers a full render cycle to fire.
	await page.waitForTimeout(500);

	// A1 — No live <img onerror> node in DOM
	log('ASSERTION', 'DOM', `[${surface}] Checking no <img onerror> node injected`);
	await expect(page.locator('img[onerror]')).toHaveCount(0);

	// A2 — No live <svg onload> node in DOM
	log('ASSERTION', 'DOM', `[${surface}] Checking no <svg onload> node injected`);
	await expect(page.locator('svg[onload]')).toHaveCount(0);

	// A3 — window.__xss_fired not set
	const xssFired = await page.evaluate(
		() => (window as unknown as Record<string, unknown>).__xss_fired,
	);
	expect(xssFired, `[${surface}] window.__xss_fired must be falsy`).toBeFalsy();
	log('VALIDATION', 'SECURITY', `[${surface}] window.__xss_fired is falsy. [PASS]`);

	// A4 — No exfiltration to tripwire host
	expect(getXssFetchMade(), `[${surface}] No exfiltration request to xss-canary.internal`).toBe(
		false,
	);
	log('VALIDATION', 'SECURITY', `[${surface}] No tripwire fetch. [PASS]`);

	// A5 — No XSS-related console errors
	const xssErrors = getConsoleErrors().filter(
		(e) =>
			e.includes('xss-canary') ||
			e.includes('onerror') ||
			e.includes('__xss') ||
			e.includes('onload'),
	);
	expect(xssErrors, `[${surface}] No XSS console errors`).toHaveLength(0);
	log('VALIDATION', 'SECURITY', `[${surface}] Zero XSS console errors. [PASS]`);
}

/** Shared mocked-auth base: trusted single device, stub servers + conversations. */
async function setupBaseAuth(page: Page, installId = 'xss-surface-install'): Promise<void> {
	const device = buildMockDevice({ installation_id: installId });
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, installId);
	await mockAuthEndpoints(page, authData);

	// Stub list endpoints so the app shell doesn't make extra real requests.
	await page.route('**/api/v2/servers', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		} else {
			await route.continue();
		}
	});

	await page.route('**/api/v2/conversations', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		} else {
			await route.continue();
		}
	});
}

// ─── Surface 1: Explore search results ───────────────────────────────────────

test.describe('XSS neutralization — explore search result usernames @security @xss @smoke', () => {
	test('XSS in search result display names is escaped, not executed', async ({ page }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(page);
		await setupBaseAuth(page);

		await mockExploreEndpoints(page, {
			searchUsers: XSS_PAYLOADS.map((payload, i) => ({
				id: `xss-user-${i}`,
				username: `xss_user_${i}`,
				displayName: payload,
			})),
		});

		log('NETWORK', 'NAVIGATE', 'Loading /explore with XSS payloads in search user display names');
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// Trigger search to populate user results
		const searchInput = page.locator('input[placeholder*="Search" i], input[type="search"]').first();
		if (await searchInput.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await searchInput.fill('xss');
			await page.waitForTimeout(500); // debounce
		}

		await assertXssNeutralized(page, 'explore-search-username', getXssFetchMade, getConsoleErrors);
	});
});

// ─── Surface 2: DM conversation list ─────────────────────────────────────────

test.describe('XSS neutralization — DM peer display names @security @xss @smoke', () => {
	test('XSS in DM peer display names is escaped, not executed', async ({ page }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(page);

		const device = buildMockDevice({ installation_id: 'xss-dm-install' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'xss-dm-install');
		await mockAuthEndpoints(page, authData);

		// Stub server list
		await page.route('**/api/v2/servers', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});

		// Inject XSS payloads into conversation peer display names
		const conversations = XSS_PAYLOADS.map((payload, i) => ({
			id: `xss-conv-${i}`,
			peer_id: `xss-peer-${i}`,
			peer_username: `xss_peer_${i}`,
			peer_display_name: payload,
			peer_avatar_url: null,
			last_message_at: new Date().toISOString(),
		}));

		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(conversations),
			});
		});

		log('NETWORK', 'NAVIGATE', 'Loading /dm with XSS payloads in peer display names');
		await page.goto('/dm');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// Wait for conversation list to render
		await page.waitForTimeout(1_000);

		await assertXssNeutralized(page, 'dm-peer-display-name', getXssFetchMade, getConsoleErrors);
	});
});

// ─── Surface 3: Server names in sidebar ──────────────────────────────────────

test.describe('XSS neutralization — server names in sidebar @security @xss @smoke', () => {
	test('XSS in server names is escaped, not executed', async ({ page }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(page);

		const device = buildMockDevice({ installation_id: 'xss-server-install' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'xss-server-install');
		await mockAuthEndpoints(page, authData);

		// Override server list to inject payloads as server names
		await page.route('**/api/v2/servers', async (route) => {
			if (route.request().method() !== 'GET') {
				await route.continue();
				return;
			}
			const servers = XSS_PAYLOADS.map((payload, i) => ({
				id: `xss-server-${i}`,
				name: payload,
				slug: `xss-server-${i}`,
				owner_id: 'xss-owner',
				icon_url: null,
				member_count: 1,
			}));
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(servers),
			});
		});

		await page.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});

		log('NETWORK', 'NAVIGATE', 'Loading /explore with XSS payloads in server names (sidebar)');
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		// Give sidebar time to render server list
		await page.waitForTimeout(1_000);

		await assertXssNeutralized(page, 'server-name-sidebar', getXssFetchMade, getConsoleErrors);
	});
});

// ─── Surface 4: Canvas poll question ─────────────────────────────────────────

test.describe('XSS neutralization — canvas poll question @security @xss @smoke', () => {
	test('XSS in canvas poll question text is escaped, not executed', async ({ page }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(page);
		await setupBaseAuth(page, 'xss-canvas-install');

		// Mock explore so we can navigate there cleanly
		await mockExploreEndpoints(page);

		// Override server list with a real-looking server
		await page.route('**/api/v2/servers', async (route) => {
			if (route.request().method() !== 'GET') {
				await route.continue();
				return;
			}
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{
						id: 'xss-canvas-server',
						name: 'XSS Canvas Server',
						slug: 'xss-canvas-server',
						owner_id: 'xss-owner',
						icon_url: null,
						member_count: 1,
					},
				]),
			});
		});

		// Inject XSS payload into canvas poll question
		await mockCanvasEndpoints(page, {
			polls: [
				{
					id: 'xss-poll-1',
					question: XSS_PAYLOAD_PRIMARY,
					options: [
						{ index: 0, text: 'Yes' },
						{ index: 1, text: 'No' },
					],
					vote_counts: { '0': 1, '1': 0 },
				},
			],
		});

		// Stub channels list
		await page.route('**/api/v2/servers/*/channels', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{ id: 'xss-ch-1', name: 'general', type: 'text', server_id: 'xss-canvas-server' },
				]),
			});
		});

		log(
			'NETWORK',
			'NAVIGATE',
			'Loading /explore (canvas loaded from server sidebar) with XSS poll question',
		);
		await page.goto('/explore');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await assertXssNeutralized(page, 'canvas-poll-question', getXssFetchMade, getConsoleErrors);
	});
});

// ─── Surface 5: Support ticket descriptions ───────────────────────────────────

test.describe(
	'XSS neutralization — support ticket descriptions @security @xss @smoke',
	() => {
		test('XSS in support ticket description text is escaped, not executed', async ({ page }) => {
			const { getXssFetchMade, getConsoleErrors } = installTripwires(page);
			await setupBaseAuth(page, 'xss-support-install');
			await mockExploreEndpoints(page);

			// Inject XSS payload into existing ticket descriptions shown in history
			await mockSupportEndpoints(page, [
				{
					id: 'xss-ticket-1',
					type: 'bug',
					priority: 'high',
					description: XSS_PAYLOAD_PRIMARY,
					createdAt: new Date().toISOString(),
				},
				{
					id: 'xss-ticket-2',
					type: 'feature',
					priority: 'low',
					description: XSS_PAYLOADS[1],
					createdAt: new Date().toISOString(),
				},
			]);

			log('NETWORK', 'NAVIGATE', 'Loading /settings with XSS payloads in support ticket history');
			await page.goto('/settings');
			await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
				timeout: 30_000,
			});

			// Navigate to Support section if settings has tabs
			const supportNav = page
				.locator('button, a, [role="tab"]')
				.filter({ hasText: /support/i })
				.first();
			if (await supportNav.isVisible({ timeout: 3_000 }).catch(() => false)) {
				await supportNav.click();
				await page.waitForTimeout(500);
			}

			await assertXssNeutralized(
				page,
				'support-ticket-description',
				getXssFetchMade,
				getConsoleErrors,
			);
		});
	},
);
