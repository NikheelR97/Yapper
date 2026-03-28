/**
 * Codex Security Report - Section 3.4 (Extended)
 * Feature: XSS / Injection Neutralization Across All Input Surfaces
 *
 * @security @xss @smoke
 */

import type { ConsoleMessage, Page, Route } from '@playwright/test';
import { test, expect } from './fixtures/auth.fixture';
import {
	mockExploreEndpoints,
	mockCanvasEndpoints,
	mockSupportEndpoints,
} from './helpers/mock-routes.js';
import { log } from './helpers/log.js';

const XSS_PAYLOADS = [
	'<img src=x onerror="window.__xss_fired=true">',
	'<script>window.__xss_fired=true</script>',
	'<svg onload="window.__xss_fired=true">',
];

const XSS_PAYLOAD_PRIMARY = XSS_PAYLOADS[0];

function installTripwires(page: Page): {
	getXssFetchMade: () => boolean;
	getConsoleErrors: () => string[];
} {
	let xssFetchMade = false;
	const consoleErrors: string[] = [];

	page.route('**xss-canary.internal**', async (route: Route) => {
		xssFetchMade = true;
		log('SECURITY', 'XSS_TRIPWIRE', `ALERT - exfiltration request: ${route.request().url()}`);
		await route.abort();
	});

	page.on('console', (msg: ConsoleMessage) => {
		if (msg.type() === 'error') {
			consoleErrors.push(msg.text());
		}
	});

	return {
		getXssFetchMade: () => xssFetchMade,
		getConsoleErrors: () => consoleErrors,
	};
}

async function assertXssNeutralized(
	page: Page,
	surface: string,
	getXssFetchMade: () => boolean,
	getConsoleErrors: () => string[],
): Promise<void> {
	log('ASSERTION', 'DOM', `[${surface}] Checking no <img onerror> node injected`);
	await expect(page.locator('img[onerror]')).toHaveCount(0);

	log('ASSERTION', 'DOM', `[${surface}] Checking no <svg onload> node injected`);
	await expect(page.locator('svg[onload]')).toHaveCount(0);

	const xssFired = await page.evaluate(
		() => (window as unknown as Record<string, unknown>).__xss_fired,
	);
	expect(xssFired, `[${surface}] window.__xss_fired must be falsy`).toBeFalsy();
	expect(getXssFetchMade(), `[${surface}] No exfiltration request to xss-canary.internal`).toBe(
		false,
	);

	const xssErrors = getConsoleErrors().filter(
		(error) =>
			error.includes('xss-canary') ||
			error.includes('onerror') ||
			error.includes('__xss') ||
			error.includes('onload'),
	);
	expect(xssErrors, `[${surface}] No XSS console errors`).toHaveLength(0);
}

async function setupBaseRoutes(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
			return;
		}
		await route.continue();
	});

	await page.route('**/api/v2/conversations', async (route) => {
		if (route.request().method() === 'GET') {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
			return;
		}
		await route.continue();
	});
}

test.describe('XSS neutralization - explore search result usernames @security @xss @smoke', () => {
	test('XSS in search result display names is escaped, not executed', async ({ userPage }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(userPage);
		await setupBaseRoutes(userPage);
		await mockExploreEndpoints(userPage, {
			searchUsers: XSS_PAYLOADS.map((payload, index) => ({
				id: `xss-user-${index}`,
				username: `xss_user_${index}`,
				displayName: payload,
			})),
		});

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		const searchInput = userPage.locator('input[placeholder*="Search" i], input[type="search"]').first();
		if (await searchInput.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await searchInput.fill('xss');
			await expect(userPage.locator('body')).toContainText('xss_user_0', { timeout: 5_000 });
		}

		await assertXssNeutralized(userPage, 'explore-search-username', getXssFetchMade, getConsoleErrors);
	});
});

test.describe('XSS neutralization - DM peer display names @security @xss @smoke', () => {
	test('XSS in DM peer display names is escaped, not executed', async ({ userPage }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(userPage);
		await setupBaseRoutes(userPage);

		const conversations = XSS_PAYLOADS.map((payload, index) => ({
			id: `xss-conv-${index}`,
			peer_id: `xss-peer-${index}`,
			peer_username: `xss_peer_${index}`,
			peer_display_name: payload,
			peer_avatar_url: null,
			last_message_at: new Date().toISOString(),
		}));

		await userPage.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify(conversations),
			});
		});

		await userPage.goto('/dm');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await expect(userPage.locator('body')).toContainText('xss_peer_0', { timeout: 5_000 });

		await assertXssNeutralized(userPage, 'dm-peer-display-name', getXssFetchMade, getConsoleErrors);
	});
});

test.describe('XSS neutralization - server names in sidebar @security @xss @smoke', () => {
	test('XSS in server names is escaped, not executed', async ({ userPage }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(userPage);

		await userPage.route('**/api/v2/servers', async (route) => {
			if (route.request().method() !== 'GET') {
				await route.continue();
				return;
			}
			const servers = XSS_PAYLOADS.map((payload, index) => ({
				id: `xss-server-${index}`,
				name: payload,
				slug: `xss-server-${index}`,
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
		await userPage.route('**/api/v2/conversations', async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await assertXssNeutralized(userPage, 'server-name-sidebar', getXssFetchMade, getConsoleErrors);
	});
});

test.describe('XSS neutralization - canvas poll question @security @xss @smoke', () => {
	test('XSS in canvas poll question text is escaped, not executed', async ({ userPage }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(userPage);
		await setupBaseRoutes(userPage);
		await mockExploreEndpoints(userPage);
		await userPage.route('**/api/v2/servers', async (route) => {
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
		await mockCanvasEndpoints(userPage, {
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
		await userPage.route('**/api/v2/servers/*/channels', async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{ id: 'xss-ch-1', name: 'general', type: 'text', server_id: 'xss-canvas-server' },
				]),
			});
		});

		await userPage.goto('/explore');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		await assertXssNeutralized(userPage, 'canvas-poll-question', getXssFetchMade, getConsoleErrors);
	});
});

test.describe('XSS neutralization - support ticket descriptions @security @xss @smoke', () => {
	test('XSS in support ticket description text is escaped, not executed', async ({ userPage }) => {
		const { getXssFetchMade, getConsoleErrors } = installTripwires(userPage);
		await setupBaseRoutes(userPage);
		await mockExploreEndpoints(userPage);
		await mockSupportEndpoints(userPage, [
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

		await userPage.goto('/settings');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});

		const supportNav = userPage
			.locator('button, a, [role="tab"]')
			.filter({ hasText: /support/i })
			.first();
		if (await supportNav.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await supportNav.click();
			await expect(userPage.getByRole('heading', { name: /Support/i })).toBeVisible({
				timeout: 5_000,
			});
		}

		await assertXssNeutralized(
			userPage,
			'support-ticket-description',
			getXssFetchMade,
			getConsoleErrors,
		);
	});
});
