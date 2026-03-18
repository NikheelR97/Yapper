/**
 * Shared wait utilities — deduplicates waitForAppReady that was previously
 * copy-pasted into servers.spec.ts, social.spec.ts, and channel-e2ee.spec.ts.
 */

import { type Page, expect } from '@playwright/test';

/**
 * Wait until the full-screen loading overlay disappears.
 * The app sets aria-label="Loading Yapper" while the boot sequence runs.
 */
export async function waitForAppReady(page: Page, timeout = 45_000): Promise<void> {
	await expect(page.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, { timeout });
}

/**
 * Wait for a toast notification containing the given text to appear.
 * Toasts are rendered by the Toast.svelte component; they use role="status" or
 * a .toast class depending on the implementation.
 */
export async function waitForToast(page: Page, text: string | RegExp, timeout = 8_000): Promise<void> {
	await expect(
		page.locator('[role="status"], .toast, [data-toast]').filter({ hasText: text }),
	).toBeVisible({ timeout });
}

/**
 * Wait for the app's WebSocket to report a connected state.
 * Relies on the .connected class or aria attribute set by the reconnection banner
 * disappearing (the banner is shown when disconnected, hidden when connected).
 */
export async function waitForWebSocketConnected(page: Page, timeout = 15_000): Promise<void> {
	// The reconnecting banner is absent when WS is healthy
	await expect(page.locator('.reconnecting-banner, [data-reconnecting]')).toHaveCount(0, {
		timeout,
	});
}
