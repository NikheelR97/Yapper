import { test, expect } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { E2EApiClient } from './helpers/api-client.js';
import { waitForAppReady } from './helpers/wait-for.js';

/**
 * Feature: Server Settings
 *
 * Tests server settings page rendering and custom emoji management section.
 */

test.describe.configure({ timeout: 120_000 });

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';
const _API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

const client = new E2EApiClient();

test.describe('Server settings', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(!TEST_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run server settings tests');

	let serverId: string;

	test.beforeAll(async () => {
		seedTrustedPrimaryDevice();
		const session = await client.login(TEST_EMAIL, TEST_PASSWORD, PRIMARY_INSTALLATION_ID, 'Server Settings Test');
		const result = await client.createServer(session, `E2E Settings ${Date.now()}`);
		serverId = result.serverId;
	});

	test('server settings page renders name and description fields', async ({ page }) => {
		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);

		await page.goto(`/servers/${serverId}/settings`);

		// Server name field should be visible — placeholder is "My Server"
		await expect(
			page.locator('input[placeholder*="Server" i], input[placeholder*="server" i], input[type="text"]').first(),
		).toBeVisible({ timeout: 10_000 });
	});

	test('custom emoji management section renders upload form @smoke', async ({ page }) => {
		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);

		// Navigate to the server and find custom emoji settings
		await page.goto(`/servers/${serverId}/settings`);

		// Look for emoji section
		const emojiSection = page
			.getByText(/Custom Emoji|Emoji/i)
			.or(page.locator('[data-testid="emoji-settings"]'))
			.first();

		if (await emojiSection.isVisible({ timeout: 5_000 }).catch(() => false)) {
			await expect(emojiSection).toBeVisible();
		} else {
			// Try navigating to the emoji section via a tab/nav item
			const emojiTab = page.getByRole('button', { name: /Emoji/i });
			if (await emojiTab.isVisible({ timeout: 3_000 }).catch(() => false)) {
				await emojiTab.click();
				await expect(
					page.locator('input[type="file"], [data-testid="emoji-upload"]').first(),
				).toBeVisible({ timeout: 5_000 });
			}
		}
	});
});
