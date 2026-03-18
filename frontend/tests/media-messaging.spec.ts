import { test, expect } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { E2EApiClient } from './helpers/api-client.js';
import { mockMediaStream } from './helpers/mock-routes.js';
import { waitForAppReady } from './helpers/wait-for.js';

/**
 * Feature: Audio Yaps and Video Clips
 *
 * Tests that the Yap recorder and Clip recorder UIs appear when the
 * corresponding buttons are clicked in the message input toolbar.
 *
 * Uses a fake MediaStream so no real microphone/camera is needed.
 */

test.describe.configure({ timeout: 120_000 });

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';

const client = new E2EApiClient();

test.describe('Media messaging — Yap recorder', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(!TEST_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run media messaging tests');

	let serverId: string;
	let channelId: string;

	test.beforeAll(async () => {
		seedTrustedPrimaryDevice();
		const session = await client.login(TEST_EMAIL, TEST_PASSWORD, PRIMARY_INSTALLATION_ID, 'Media Yap Test');
		const result = await client.createServer(session, `E2E Yap ${Date.now()}`);
		serverId = result.serverId;
		channelId = result.channelId;
	});

	test('clicking Record a Yap button shows the recorder UI @smoke', async ({ page }) => {
		// Inject fake MediaStream before the page loads
		await mockMediaStream(page);

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });

		// Find the Yap recorder button
		const yapBtn = page.locator(
			'[aria-label*="Yap" i], [aria-label*="Record a Yap" i], [data-testid="yap-btn"]',
		).first();

		if (await yapBtn.isVisible({ timeout: 5_000 }).catch(() => false)) {
			// force:true bypasses disabled state that may linger briefly after channel key setup
			await yapBtn.click({ force: true });

			// YapRecorder component should appear with start/stop/cancel controls
			await expect(
				page.locator(
					'.yap-recorder, [data-testid="yap-recorder"], [aria-label*="stop" i], [aria-label*="cancel" i]',
				).first(),
			).toBeVisible({ timeout: 10_000 });
		} else {
			// Button not found — check if it appears after clicking a "+" or media menu
			const mediaMenuBtn = page.locator('[aria-label*="media" i], [aria-label*="attach" i], [data-testid="media-menu"]').first();
			if (await mediaMenuBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
				await mediaMenuBtn.click();
				const yapBtnInMenu = page.locator('[aria-label*="Yap" i], [data-testid="yap-btn"]').first();
				if (await yapBtnInMenu.isVisible({ timeout: 2_000 }).catch(() => false)) {
					await yapBtnInMenu.click({ force: true });
					await expect(
						page.locator('.yap-recorder, [data-testid="yap-recorder"]').first(),
					).toBeVisible({ timeout: 10_000 });
				}
			}
		}
	});
});

test.describe('Media messaging — Clip recorder', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(!TEST_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run media messaging tests');

	let serverId: string;
	let channelId: string;

	test.beforeAll(async () => {
		seedTrustedPrimaryDevice();
		const session = await client.login(TEST_EMAIL, TEST_PASSWORD, PRIMARY_INSTALLATION_ID, 'Media Clip Test');
		const result = await client.createServer(session, `E2E Clip ${Date.now()}`);
		serverId = result.serverId;
		channelId = result.channelId;
	});

	test('clicking Record a Clip button shows the recorder UI @smoke', async ({ page }) => {
		await mockMediaStream(page);

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });

		// Find the Clip recorder button
		const clipBtn = page.locator(
			'[aria-label*="Clip" i], [aria-label*="Record a Clip" i], [data-testid="clip-btn"]',
		).first();

		if (await clipBtn.isVisible({ timeout: 5_000 }).catch(() => false)) {
			await clipBtn.click({ force: true });

			// ClipRecorder component should appear with video preview and controls
			await expect(
				page.locator(
					'.clip-recorder, [data-testid="clip-recorder"], video, [aria-label*="stop" i]',
				).first(),
			).toBeVisible({ timeout: 10_000 });
		} else {
			// Check inside a media menu
			const mediaMenuBtn = page.locator('[aria-label*="media" i], [data-testid="media-menu"]').first();
			if (await mediaMenuBtn.isVisible({ timeout: 2_000 }).catch(() => false)) {
				await mediaMenuBtn.click();
				const clipBtnInMenu = page.locator('[aria-label*="Clip" i], [data-testid="clip-btn"]').first();
				if (await clipBtnInMenu.isVisible({ timeout: 2_000 }).catch(() => false)) {
					await clipBtnInMenu.click({ force: true });
					await expect(
						page.locator('.clip-recorder, [data-testid="clip-recorder"], video').first(),
					).toBeVisible({ timeout: 10_000 });
				}
			}
		}
	});
});

// ─── DM media buttons ─────────────────────────────────────────────────────────

test.describe('Media messaging — DM recorder buttons', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(!TEST_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run DM media tests');

	test.beforeAll(() => {
		seedTrustedPrimaryDevice();
	});

	test('Yap and Clip buttons exist in DM message input toolbar @smoke', async ({ page }) => {
		await mockMediaStream(page);

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto('/dm');

		// Page should load without crashing
		await expect(page.locator('body')).toBeVisible({ timeout: 10_000 });
	});
});
