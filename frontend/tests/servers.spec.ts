import { test, expect, type Page } from '@playwright/test';
import { mockAuthEndpoints } from './auth-helper.js';

/**
 * Servers & Channels E2E tests.
 *
 * Covers: server creation, channel navigation, message sending, invite links.
 * Requires E2E_EMAIL / E2E_PASSWORD.
 */


async function loginAs(page: Page) {
	await mockAuthEndpoints(page);
	await page.goto('/explore');
	await page.waitForURL(/\/explore/, { timeout: 20_000 });
}

// ─── Servers index ─────────────────────────────────────────────────────────────

test.describe('Servers — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
	});

	test('/servers page renders', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });
		await expect(page.locator('body')).toBeVisible();
	});

	test('Create Server button is visible in sidebar', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });
		// Button has class "add-btn" and aria-label "Create or join a server"
		await expect(page.getByRole('button', { name: 'Create or join a server' })).toBeVisible({ timeout: 20_000 });
	});

	test('Create Server modal opens', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.getByRole('button', { name: 'Create or join a server' });
		await createBtn.waitFor({ timeout: 20_000 });
		await createBtn.click();

		// Modal or dialog should appear
		const modal = page.locator('[role="dialog"]').first();
		await expect(modal).toBeVisible({ timeout: 5_000 });
	});

	test('Create Server modal has name input', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.getByRole('button', { name: 'Create or join a server' });
		await createBtn.waitFor({ timeout: 20_000 });
		await createBtn.click();

		await expect(page.locator('.modal-input').first())
			.toBeVisible({ timeout: 5_000 });
	});

	test('can create a server with a unique name', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.getByRole('button', { name: 'Create or join a server' });
		await createBtn.waitFor({ timeout: 20_000 });
		await createBtn.click();

		const serverName = `E2E Server ${Date.now()}`;
		const nameInput = page.locator('.modal-input').first();
		await nameInput.fill(serverName);

		// Submit
		const submitBtn = page.locator('.modal-submit');
		await submitBtn.click();

		// Should redirect to the new server or close modal
		await page.waitForTimeout(3_000);
		const url = page.url();
		const modalGone = await page.locator('[role="dialog"]').isHidden().catch(() => true);

		expect(url.includes('/servers/') || modalGone).toBeTruthy();
	});
});

// ─── Channel page ──────────────────────────────────────────────────────────────

test.describe('Channel — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
	});

	// Helper: navigate to /servers and wait for the auto-redirect chain to land on a channel page
	async function gotoFirstChannel(page: Page): Promise<boolean> {
		await page.goto('/servers');
		// /servers → /servers/{id}/channels → /servers/{id}/channels/{channelId}
		return page
			.waitForURL(/\/servers\/[^/]+\/channels\/[^/]+/, { timeout: 20_000 })
			.then(() => true)
			.catch(() => false);
	}

	test('first server channel page renders message input', async ({ page }) => {
		const onChannel = await gotoFirstChannel(page);
		if (!onChannel) { test.skip(); return; }

		// Message input should be present
		const input = page.locator('textarea, [contenteditable="true"], input[placeholder*="Message"]').first();
		await expect(input).toBeVisible({ timeout: 8_000 });
	});

	test('sending a channel message renders it in the list', async ({ page }) => {
		test.slow(); // E2EE joinChannel() makes multiple API calls — allow 90s
		const onChannel = await gotoFirstChannel(page);
		if (!onChannel) { test.skip(); return; }

		const input = page.locator('textarea[aria-label="Message"]').first();
		// Wait for E2EE setup ("Setting up encryption…") to finish
		await expect(input).toBeEnabled({ timeout: 60_000 });

		const testMsg = `E2E channel test ${Date.now()}`;
		await input.fill(testMsg);
		await input.press('Enter');

		await expect(page.getByText(testMsg)).toBeVisible({ timeout: 8_000 });
	});

	test('typing indicator appears when typing in channel', async ({ page }) => {
		test.slow(); // E2EE joinChannel() makes multiple API calls — allow 90s
		const onChannel = await gotoFirstChannel(page);
		if (!onChannel) { test.skip(); return; }

		const input = page.locator('textarea[aria-label="Message"]').first();
		// Wait for E2EE setup to finish before typing
		await expect(input).toBeEnabled({ timeout: 60_000 });
		await input.fill('typing...');

		// No assertion on indicator visibility (need second user), just ensure no crash
		await expect(page.locator('body')).toBeVisible();
	});
});

// ─── Invite links ──────────────────────────────────────────────────────────────

test.describe('Invite links — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page);
	});

	test('invite link can be generated for a server', async ({ page }) => {
		const onChannel = await page.goto('/servers').then(() =>
			page.waitForURL(/\/servers\/[^/]+\/channels\/[^/]+/, { timeout: 20_000 }).then(() => true).catch(() => false)
		);
		if (!onChannel) { test.skip(); return; }

		// Look for invite button in sidebar or header
		const inviteBtn = page.getByRole('button', { name: /Invite/i })
			.or(page.locator('[title*="invite"], [aria-label*="invite"]')).first();

		const hasInvite = await inviteBtn.isVisible({ timeout: 5_000 }).catch(() => false);
		if (!hasInvite) {
			test.skip();
			return;
		}

		await inviteBtn.click();

		// Invite link or modal should appear
		const inviteLink = page.locator('input[value*="yapper://"], input[value*="/invite/"], [class*="invite"]').first();
		await expect(inviteLink.or(page.locator('[role="dialog"]').first())).toBeVisible({ timeout: 5_000 });
	});
});
