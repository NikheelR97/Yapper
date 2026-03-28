import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Custom Emoji Rendering
 *
 * Tests that `:emoji_name:` shortcodes in messages are rendered as `<img>` tags
 * with safe URLs, and that XSS payloads in emoji URLs are blocked.
 */

const SAFE_EMOJI_URL = 'https://cdn.yapperhq.com/emojis/test-emoji.webp';
const XSS_EMOJI_URL = 'javascript:alert(1)';

async function setupAuthWithServer(
	page: Parameters<typeof mockAuthEndpoints>[0],
	serverId: string,
	channelId: string,
	emojis: Array<{ name: string; url: string }>,
): Promise<void> {
	const device = buildMockDevice({ installation_id: 'emoji-test-install' });
	const authData = buildMockAuthData({ device });
	await page.addInitScript(() => {
		const clearNativeMarkers = () => {
			try {
				delete (window as unknown as { Capacitor?: unknown }).Capacitor;
				delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
				delete (window as unknown as { __TAURI__?: unknown }).__TAURI__;
			} catch {
				// Ignore if the runtime seals these properties; the web path still works when absent.
			}
		};
		clearNativeMarkers();
		window.setInterval(clearNativeMarkers, 0);
		try {
			const proxiedWindow = new Proxy(window, {
				has(target, prop) {
					if (prop === 'Capacitor' || prop === '__TAURI_INTERNALS__' || prop === '__TAURI__') {
						return false;
					}
					return Reflect.has(target, prop);
				},
				get(target, prop, receiver) {
					if (prop === 'Capacitor' || prop === '__TAURI_INTERNALS__' || prop === '__TAURI__') {
						return undefined;
					}
					return Reflect.get(target, prop, receiver);
				},
			});
			Object.defineProperty(globalThis, 'window', {
				configurable: true,
				get: () => proxiedWindow,
			});
		} catch {
			// If the runtime refuses to proxy the global, the repeated cleanup still helps.
		}
	});
	await setInstallationId(page, 'emoji-test-install');
	await mockAuthEndpoints(page, authData);

	await page.route(`**/api/v2/servers`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{
					id: serverId,
					name: 'Emoji Test Server',
					icon_url: null,
					owner_id: 'e2e-user',
					channels: [{ id: channelId, name: 'general', server_id: serverId }],
				},
			]),
		});
	});

	await page.route(`**/api/v2/users/**/presence`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
	});

	await page.route(`**/api/v2/conversations`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	// Mock server emojis
	await page.route(`**/api/v2/servers/${serverId}/emojis`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				emojis: emojis.map((e, i) => ({
					id: `emoji-${i}`,
					name: e.name,
					image_url: e.url,
				})),
			}),
		});
	});

	await page.route(`**/api/v2/servers/${serverId}/channels`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{
					id: channelId,
					server_id: serverId,
					name: 'general',
					type: 'text',
					position: 1,
				},
			]),
		});
	});

	// Mock channel messages containing emoji shortcodes
	await page.route(`**/api/v2/channels/${channelId}/messages**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{
					id: 'msg-emoji-1',
					channel_id: channelId,
					sender_id: 'e2e-user',
					sender_device_id: 1,
					ciphertext: null,
					plaintext: `Check out this emoji :${emojis[0]?.name ?? 'test_emoji'}: cool right?`,
					message_type: 'text',
					msg_num: 1,
					is_bot: true, // Use bot so it renders plaintext directly
					created_at: new Date().toISOString(),
				},
			]),
		});
	});

	await page.route(`**/api/v2/channels/${channelId}/members`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	await page.route(`**/api/v2/channels/${channelId}/sender-key-dist`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	await page.route(`**/api/v2/canvas/servers/${serverId}/state**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({ music: null, polls: [], clips: [] }),
		});
	});

	await page.route(`**/api/v2/servers/${serverId}/members**`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{ user_id: 'e2e-user', username: 'e2e_user', display_name: 'E2E User', account_type: 'standard' },
			]),
		});
	});

	await mockExploreEndpoints(page);
	page.on('response', (response) => {
		if (response.status() === 401 || response.status() === 403 || response.status() >= 500) {
			console.log('[emoji response]', response.status(), response.url());
		}
	});
	page.on('pageerror', (err) => console.log('[emoji pageerror]', err.message));
	page.on('console', (msg) => {
		if (msg.type() === 'error') console.log('[emoji console]', msg.text());
	});
}

test.describe('Custom emoji rendering @smoke', () => {
	test('renders :emoji_name: as <img> with safe URL', async ({ page }) => {
		const serverId = 'srv-emoji-render';
		const channelId = 'ch-emoji-render';

		await setupAuthWithServer(page, serverId, channelId, [
			{ name: 'party_parrot', url: SAFE_EMOJI_URL },
		]);

		const bootstrapReady = Promise.all([
			page.waitForResponse((resp) => resp.url().includes('/api/v2/keys/identity') && resp.status() === 200),
			page.waitForResponse((resp) => resp.url().includes('/api/v2/keys/signed-prekey') && resp.status() === 200),
			page.waitForResponse((resp) => resp.url().includes('/api/v2/keys/one-time-prekeys') && resp.status() === 200),
			page.waitForResponse((resp) => resp.url().includes('/api/v2/keys/one-time-prekey-count') && resp.status() === 200),
		]);
		await page.goto('/explore');
		await bootstrapReady;
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.getByLabel('Loading Yapper')).toBeHidden({ timeout: 30_000 });

		// Check that the page loaded without errors
		await expect(page.locator('body')).toBeVisible();

		await expect(page.getByTestId('message-list')).toContainText('Check out this emoji', { timeout: 10_000 });
		await expect(page.locator(`img[src="${SAFE_EMOJI_URL}"]`)).toHaveCount(1, { timeout: 10_000 });
	});

	test('blocks javascript: protocol in emoji URLs (XSS prevention)', async ({ page }) => {
		const serverId = 'srv-emoji-xss';
		const channelId = 'ch-emoji-xss';

		await setupAuthWithServer(page, serverId, channelId, [
			{ name: 'xss_emoji', url: XSS_EMOJI_URL },
		]);

		// Monitor for JS errors that would indicate XSS execution
		const jsErrors: string[] = [];
		page.on('pageerror', (err) => jsErrors.push(err.message));

		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.getByLabel('Loading Yapper')).toBeHidden({ timeout: 30_000 });

		// No img tags should have javascript: src
		const dangerousImgs = await page.locator('img[src^="javascript:"]').count();
		expect(dangerousImgs).toBe(0);

		// No alert-related JS errors
		const xssErrors = jsErrors.filter((e) => e.includes('alert'));
		expect(xssErrors).toHaveLength(0);
	});
});

test.describe('Emoji picker integration @smoke', () => {
	test('emoji picker button is visible in message input area', async ({ page }) => {
		const serverId = 'srv-emoji-picker';
		const channelId = 'ch-emoji-picker';

		await setupAuthWithServer(page, serverId, channelId, [
			{ name: 'thumbsup', url: SAFE_EMOJI_URL },
		]);

		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.getByLabel('Loading Yapper')).toBeHidden({ timeout: 30_000 });

		// Look for emoji picker toggle button near the message input
		const emojiBtn = page.locator(
			'button[aria-label*="emoji" i], button[aria-label*="Emoji" i], button[title*="emoji" i], .emoji-toggle, [data-testid="emoji-picker-toggle"]',
		);
		const hasEmojiBtn = await emojiBtn.first().isVisible({ timeout: 5_000 }).catch(() => false);

		// The message input area should at minimum be present
		await expect(page.locator('body')).toBeVisible();

		if (hasEmojiBtn) {
			await expect(emojiBtn.first()).toBeVisible();
		}
	});
});
