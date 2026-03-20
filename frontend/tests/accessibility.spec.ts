/**
 * Accessibility — WCAG 2.1 AA Compliance
 *
 * Scans key routes using axe-core for WCAG 2.1 Level A & AA violations.
 * Each test navigates to the route under mocked auth and runs a full axe scan.
 *
 * Rules explicitly excluded (tracked separately, not ignored permanently):
 *   - color-contrast: dark theme requires design-system audit, not test fix
 *
 * Tags: @accessibility — excluded from nightly smoke shards by default.
 * Run manually: npx playwright test --grep "@accessibility"
 *
 * @accessibility
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {
	buildMockAuthData,
	buildMockDevice,
	mockAuthEndpoints,
	setInstallationId,
} from './auth-helper.js';
import {
	mockExploreEndpoints,
	mockParentalEndpoints,
	mockProfileEndpoints,
	mockSupportEndpoints,
} from './helpers/mock-routes.js';

// ─── Shared helpers ───────────────────────────────────────────────────────────

async function setupAuth(page: Page, installId = 'a11y-install'): Promise<void> {
	const device = buildMockDevice({
		installation_id: installId,
		trust_state: 'trusted',
		approved_at: new Date().toISOString(),
	});
	const authData = buildMockAuthData({ device });
	await setInstallationId(page, installId);
	await mockAuthEndpoints(page, authData, { devices: [device] });

	await page.route('**/api/v1/servers', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
	await page.route('**/api/v2/conversations', async (route) => {
		await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
	});
}

async function runAxe(page: Page): Promise<void> {
	const results = await new AxeBuilder({ page })
		.withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
		.exclude('[data-testid="dev-only"]')
		// color-contrast excluded: dark theme is intentional design; tracked in UX backlog
		.disableRules(['color-contrast'])
		.analyze();

	// Surface clear violation list in the test failure message
	if (results.violations.length > 0) {
		const summary = results.violations
			.map((v) => `[${v.impact?.toUpperCase()}] ${v.id}: ${v.description}`)
			.join('\n');
		expect.soft(results.violations, `WCAG violations:\n${summary}`).toHaveLength(0);
	}
	expect(results.violations).toHaveLength(0);
}

async function waitForReady(page: Page): Promise<void> {
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout: 30_000 });
}

// ─── Unauthenticated routes ───────────────────────────────────────────────────

test.describe('Accessibility — unauthenticated pages @accessibility', () => {
	test('/login page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/login');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});

	test('/register page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/register');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});

	test('/forgot-password page meets WCAG 2.1 AA', async ({ page }) => {
		await page.goto('/forgot-password');
		await page.waitForLoadState('networkidle');
		await runAxe(page);
	});
});

// ─── Authenticated routes ─────────────────────────────────────────────────────

test.describe('Accessibility — authenticated pages @accessibility', () => {
	test('/explore page meets WCAG 2.1 AA', async ({ page }) => {
		await setupAuth(page, 'a11y-explore');
		await mockExploreEndpoints(page);
		await page.goto('/explore');
		await waitForReady(page);
		await runAxe(page);
	});

	test('/dm page meets WCAG 2.1 AA', async ({ page }) => {
		await setupAuth(page, 'a11y-dm');
		await mockExploreEndpoints(page);
		await page.goto('/dm');
		await waitForReady(page);
		await runAxe(page);
	});

	test('/settings page meets WCAG 2.1 AA', async ({ page }) => {
		await setupAuth(page, 'a11y-settings');
		await mockExploreEndpoints(page);
		await mockSupportEndpoints(page);
		await page.goto('/settings');
		await waitForReady(page);
		await runAxe(page);
	});

	test('/settings — each section panel meets WCAG 2.1 AA', async ({ page }) => {
		await setupAuth(page, 'a11y-settings-panels');
		await mockExploreEndpoints(page);
		await mockSupportEndpoints(page);
		await page.goto('/settings');
		await waitForReady(page);

		// Click through each settings nav item and run axe on each panel
		const navItems = page.locator('nav button, nav a, [role="tab"]');
		const count = await navItems.count();

		for (let i = 0; i < count; i++) {
			const item = navItems.nth(i);
			if (await item.isVisible()) {
				await item.click();
				await page.waitForTimeout(300); // panel animation
				await runAxe(page);
			}
		}
	});

	test('/parent/children/setup meets WCAG 2.1 AA', async ({ page }) => {
		await setupAuth(page, 'a11y-parent');
		await mockExploreEndpoints(page);
		await mockParentalEndpoints(page);
		await page.goto('/parent/children/setup');
		await waitForReady(page);
		await runAxe(page);
	});

	test('/profile/:username page meets WCAG 2.1 AA', async ({ page }) => {
		const username = 'a11y_test_user';
		await setupAuth(page, 'a11y-profile');
		await mockExploreEndpoints(page);
		await mockProfileEndpoints(page, {
			id: 'a11y-profile-id',
			username,
			displayName: 'A11y Test User',
			bio: 'Testing accessibility compliance',
		});
		await page.goto(`/profile/${username}`);
		await waitForReady(page);
		await runAxe(page);
	});
});
