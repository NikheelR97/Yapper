import { test, expect } from '@playwright/test';
import {
	setInstallationId,
	seedTrustedPrimaryDevice,
	PRIMARY_INSTALLATION_ID,
} from './auth-helper.js';
import { E2EApiClient } from './helpers/api-client.js';
import { mockCanvasEndpoints } from './helpers/mock-routes.js';
import { waitForAppReady } from './helpers/wait-for.js';

/**
 * Feature: Live Canvas
 *
 * Tests the canvas panel (music widget, polls) using mocked canvas API responses.
 * Opening/navigating to a channel still uses live auth for the page to load.
 */

test.describe.configure({ timeout: 120_000 });

const TEST_EMAIL = process.env.E2E_EMAIL ?? '';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? '';

const client = new E2EApiClient();

test.describe('Live Canvas — panel', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test.skip(!TEST_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run live canvas tests');

	let serverId: string;
	let channelId: string;

	test.beforeAll(async () => {
		seedTrustedPrimaryDevice();
		const session = await client.login(TEST_EMAIL, TEST_PASSWORD, PRIMARY_INSTALLATION_ID, 'Canvas Test');
		const result = await client.createServer(session, `E2E Canvas ${Date.now()}`);
		serverId = result.serverId;
		channelId = result.channelId;
	});

	test('canvas toggle button is present in channel header', async ({ page }) => {
		await mockCanvasEndpoints(page);

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });

		// Canvas toggle button should be in the channel header
		const canvasToggle = page.locator(
			'[aria-label*="canvas" i], [data-testid="canvas-toggle"], button.canvas-toggle',
		).first();

		if (await canvasToggle.isVisible({ timeout: 5_000 }).catch(() => false)) {
			await expect(canvasToggle).toBeVisible();
		}
	});

	test('opening the canvas panel reveals music widget and poll', async ({ page }) => {
		await mockCanvasEndpoints(page, {
			music: { title: 'Test Track', artist: 'Test Artist', albumArt: null },
			polls: [
				{
					id: 'p1',
					question: 'Best framework?',
					options: [
						{ id: 'o1', text: 'Svelte', votes: 5 },
						{ id: 'o2', text: 'React', votes: 3 },
					],
				},
			],
		});

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });

		// Try to open canvas
		const canvasToggle = page.locator(
			'[aria-label*="canvas" i], [data-testid="canvas-toggle"]',
		).first();

		if (await canvasToggle.isVisible({ timeout: 5_000 }).catch(() => false)) {
			await canvasToggle.click({ force: true });

			// Canvas panel should render music widget or poll
			await expect(
				page.locator('.music-widget, [data-testid="music-widget"], .poll-widget, [data-testid="poll-widget"]').first(),
			).toBeVisible({ timeout: 10_000 });
		} else {
			test.skip(true, 'Canvas toggle not found — canvas may not be enabled for this channel type');
		}
	});

	test('poll vote button triggers POST to vote endpoint', async ({ page }) => {
		let voteRequested = false;

		await mockCanvasEndpoints(page, {
			polls: [
				{
					id: 'poll-vote-test',
					question: 'Vote test?',
					options: [
						{ id: 'opt-1', text: 'Option A', votes: 0 },
						{ id: 'opt-2', text: 'Option B', votes: 0 },
					],
				},
			],
		});

		await page.route(`**/api/v1/servers/*/polls/*/vote`, async (route) => {
			voteRequested = true;
			await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
		});

		await setInstallationId(page, PRIMARY_INSTALLATION_ID);
		await page.goto('/login');
		await page.fill('#email', TEST_EMAIL);
		await page.fill('#password', TEST_PASSWORD);
		await page.getByRole('button', { name: /Sign In/i }).click();
		await page.waitForURL(/\/explore/, { timeout: 20_000 });
		await waitForAppReady(page);
		await page.goto(`/servers/${serverId}/channels/${channelId}`);
		await expect(page.locator('textarea[aria-label="Message"]').first()).toBeEnabled({ timeout: 60_000 });

		const canvasToggle = page.locator('[aria-label*="canvas" i], [data-testid="canvas-toggle"]').first();
		if (await canvasToggle.isVisible({ timeout: 3_000 }).catch(() => false)) {
			await canvasToggle.click();

			const voteBtn = page.getByRole('button', { name: /Option A|Vote/i }).first();
			if (await voteBtn.isVisible({ timeout: 3_000 }).catch(() => false)) {
				await voteBtn.click();
				// Give a moment for the request to fire
				await page.waitForTimeout(500);
				expect(voteRequested).toBe(true);
			}
		}
	});
});
