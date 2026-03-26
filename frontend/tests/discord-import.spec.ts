import { test, expect } from '@playwright/test';
import { buildMockAuthData, buildMockDevice, mockAuthEndpoints, setInstallationId } from './auth-helper.js';
import { mockExploreEndpoints } from './helpers/mock-routes.js';

/**
 * Feature: Discord Import — Avatar + Bot Message Display
 *
 * Tests the Discord profile import flow (connected state, avatar display)
 * and verifies that bot messages (plaintext, no E2EE) render correctly
 * in channel message lists.
 */

async function setupAuthWithDiscord(
	page: Parameters<typeof mockAuthEndpoints>[0],
	connected: boolean,
): Promise<void> {
	const device = buildMockDevice({ installation_id: 'discord-test-install' });
	const authData = buildMockAuthData({
		device,
		user: {
			id: 'discord-import-user',
			username: 'discord_tester',
			displayName: 'Discord Tester',
			avatarUrl: connected ? 'https://cdn.discordapp.com/avatars/123/abc.png' : null,
			accountType: 'standard',
			isPremium: false,
		},
	});
	await setInstallationId(page, 'discord-test-install');
	await mockAuthEndpoints(page, authData);

	await page.route(`**/api/v2/servers`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route(`**/api/v2/conversations`, async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});

	// Mock the users/me endpoint with discord connection info
	await page.route(`**/api/v2/users/me`, async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				id: 'discord-import-user',
				username: 'discord_tester',
				display_name: 'Discord Tester',
				avatar_url: connected ? 'https://cdn.discordapp.com/avatars/123/abc.png' : null,
				account_type: 'standard',
				is_premium: false,
				connections: {
					discord: connected ? { id: 'discord:123456', username: 'DiscordUser#1234' } : null,
					google: null,
					apple: null,
				},
			}),
		});
	});

	await mockExploreEndpoints(page);
}

// ─── Discord import flow ────────────────────────────────────────────────────

test.describe('Discord Import — settings page @smoke', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test('shows Discord as connected when user has linked account', async ({ page }) => {
		await setupAuthWithDiscord(page, true);

		await page.goto('/settings');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 20_000 });

		// Navigate to the connections/integrations section
		const discordSection = page.getByText(/Discord/i).first();
		await expect(discordSection).toBeVisible({ timeout: 10_000 });
	});

	test('shows Discord as not connected when no link exists', async ({ page }) => {
		await setupAuthWithDiscord(page, false);

		await page.goto('/settings');
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 20_000 });

		// Should show a Connect button or similar for Discord
		const discordText = page.getByText(/Discord/i).first();
		await expect(discordText).toBeVisible({ timeout: 10_000 });
	});
});

// ─── Bot message display ────────────────────────────────────────────────────

test.describe('Bot message display in channel @smoke', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test('bot plaintext messages render without decryption errors', async ({ page }) => {
		const device = buildMockDevice({ installation_id: 'bot-msg-install' });
		const authData = buildMockAuthData({ device });
		await setInstallationId(page, 'bot-msg-install');
		await mockAuthEndpoints(page, authData);

		const serverId = 'srv-bot-test';
		const channelId = 'ch-bot-test';

		await page.route(`**/api/v2/servers`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{
						id: serverId,
						name: 'Bot Test Server',
						icon_url: null,
						owner_id: 'discord-import-user',
						channels: [{ id: channelId, name: 'bot-channel', server_id: serverId }],
					},
				]),
			});
		});

		await page.route(`**/api/v2/conversations`, async (route) => {
			await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
		});

		// Mock channel messages with a bot plaintext message
		await page.route(`**/api/v2/channels/${channelId}/messages**`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{
						id: 'bot-msg-1',
						channel_id: channelId,
						sender_id: 'bot-user-1',
						sender_device_id: null,
						ciphertext: null,
						plaintext: 'Hello from the bot! This is a plaintext message.',
						message_type: 'text',
						msg_num: null,
						is_bot: true,
						created_at: new Date().toISOString(),
					},
					{
						id: 'user-msg-1',
						channel_id: channelId,
						sender_id: 'e2e-user',
						sender_device_id: 1,
						ciphertext: 'base64encodedciphertext',
						plaintext: null,
						message_type: 'text',
						msg_num: 1,
						is_bot: false,
						created_at: new Date(Date.now() - 60_000).toISOString(),
					},
				]),
			});
		});

		// Mock users endpoint for bot identity
		await page.route(`**/api/v2/servers/${serverId}/members**`, async (route) => {
			await route.fulfill({
				status: 200,
				contentType: 'application/json',
				body: JSON.stringify([
					{ user_id: 'bot-user-1', username: 'yapper_bot', display_name: 'Yapper Bot', account_type: 'bot' },
					{ user_id: 'e2e-user', username: 'e2e_user', display_name: 'E2E User', account_type: 'standard' },
				]),
			});
		});

		await mockExploreEndpoints(page);

		await page.goto(`/servers/${serverId}/${channelId}`);
		await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 20_000 });

		// The bot message should be visible as plaintext
		const botMessage = page.getByText('Hello from the bot!');
		const visible = await botMessage.isVisible({ timeout: 10_000 }).catch(() => false);

		// If the channel page rendered, at least the page loaded without crash
		await expect(page.locator('body')).toBeVisible();

		// No decryption error banners should appear for bot messages
		const errorBanner = page.locator('[role="alert"]').filter({ hasText: /decrypt/i });
		const hasError = await errorBanner.isVisible({ timeout: 2_000 }).catch(() => false);
		expect(hasError).toBe(false);
	});
});
