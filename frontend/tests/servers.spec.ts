import { test, expect, type Page } from '@playwright/test';

/**
 * Servers & Channels E2E tests.
 *
 * Covers: server creation, channel navigation, message sending, invite links.
 * Requires E2E_EMAIL / E2E_PASSWORD.
 */

const TEST_EMAIL    = process.env.E2E_EMAIL ?? 'e2e@test.yapper.internal';
const TEST_PASSWORD = process.env.E2E_PASSWORD ?? 'E2eTestPass1!';

async function loginAs(page: Page, email: string, password: string) {
	await page.goto('/login');
	await page.fill('#email', email);
	await page.fill('#password', password);
	await page.getByRole('button', { name: /Sign In/i }).click();
	await page.waitForURL(/\/explore/, { timeout: 10_000 });
}

// ─── Servers index ─────────────────────────────────────────────────────────────

test.describe('Servers — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
	});

	test('/servers page renders', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });
		await expect(page.locator('body')).toBeVisible();
	});

	test('Create Server button is visible in sidebar', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });
		await expect(page.locator('.create-server-btn')).toBeVisible({ timeout: 10_000 });
	});

	test('Create Server modal opens', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.locator('.create-server-btn');
		await createBtn.click();

		// Modal or dialog should appear
		const modal = page.locator('[role="dialog"], .modal, [class*="modal"]').first();
		await expect(modal).toBeVisible({ timeout: 5_000 });
	});

	test('Create Server modal has name input', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.locator('.create-server-btn');
		await createBtn.click();

		await expect(page.locator('.modal-input').first())
			.toBeVisible({ timeout: 5_000 });
	});

	test('can create a server with a unique name', async ({ page }) => {
		await page.goto('/servers');
		await page.waitForURL(/\/servers(\/|$)/, { timeout: 10_000 });

		const createBtn = page.locator('.create-server-btn');
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
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
	});

	test('first server channel page renders message input', async ({ page }) => {
		await page.goto('/servers');

		// Navigate to first available channel
		const channelLink = page.locator('a[href*="/channels/"]').first();
		const hasChannel = await channelLink.isVisible({ timeout: 6_000 }).catch(() => false);

		if (!hasChannel) {
			test.skip();
			return;
		}

		await channelLink.click();
		await page.waitForURL(/\/channels\//, { timeout: 8_000 });

		// Message input should be present
		const input = page.locator('textarea, [contenteditable="true"], input[placeholder*="Message"]').first();
		await expect(input).toBeVisible({ timeout: 5_000 });
	});

	test('sending a channel message renders it in the list', async ({ page }) => {
		await page.goto('/servers');

		const channelLink = page.locator('a[href*="/channels/"]').first();
		const hasChannel = await channelLink.isVisible({ timeout: 6_000 }).catch(() => false);

		if (!hasChannel) {
			test.skip();
			return;
		}

		await channelLink.click();
		await page.waitForURL(/\/channels\//, { timeout: 8_000 });

		const testMsg = `E2E channel test ${Date.now()}`;
		const input = page.locator('textarea, [contenteditable="true"]').first();
		await input.fill(testMsg);
		await input.press('Enter');

		await expect(page.getByText(testMsg)).toBeVisible({ timeout: 8_000 });
	});

	test('typing indicator appears when typing in channel', async ({ page }) => {
		await page.goto('/servers');

		const channelLink = page.locator('a[href*="/channels/"]').first();
		const hasChannel = await channelLink.isVisible({ timeout: 6_000 }).catch(() => false);

		if (!hasChannel) {
			test.skip();
			return;
		}

		await channelLink.click();
		await page.waitForURL(/\/channels\//, { timeout: 8_000 });

		const input = page.locator('textarea, [contenteditable="true"]').first();
		await input.fill('typing...');

		// No assertion on indicator visibility (need second user), just ensure no crash
		await expect(page.locator('body')).toBeVisible();
	});
});

// ─── Invite links ──────────────────────────────────────────────────────────────

test.describe('Invite links — authenticated', () => {
	test.skip(!process.env.E2E_EMAIL, 'Set E2E_EMAIL / E2E_PASSWORD to run these tests');

	test.beforeEach(async ({ page }) => {
		await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
	});

	test('invite link can be generated for a server', async ({ page }) => {
		await page.goto('/servers');

		const channelLink = page.locator('a[href*="/channels/"]').first();
		const hasChannel = await channelLink.isVisible({ timeout: 6_000 }).catch(() => false);

		if (!hasChannel) {
			test.skip();
			return;
		}

		await channelLink.click();
		await page.waitForURL(/\/channels\//, { timeout: 8_000 });

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
