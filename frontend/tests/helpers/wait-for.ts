/**
 * Shared wait utilities — deduplicates waitForAppReady that was previously
 * copy-pasted into servers.spec.ts, social.spec.ts, and channel-e2ee.spec.ts.
 */

import { type Page, expect } from '@playwright/test';

const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';

interface KeyBundleMinimal {
	identity_sig_key: string;
	signed_prekey: string;
	signed_prekey_sig: string;
}

/**
 * Poll until the given user has at least one valid trusted key bundle on the
 * server.  Necessary because initializeSignalKeys() runs as a background
 * promise after the app loading screen hides (boot-perf fix), so encryptDm /
 * prepareChannel may race with the key upload if called immediately after
 * waitForAppReady().
 *
 * maxWaitMs defaults to 45 000 ms (45 attempts × 1 s).
 */
export async function waitForKeyBundles(
	authToken: string,
	userId: string,
	maxWaitMs = 45_000,
): Promise<void> {
	const attempts = Math.ceil(maxWaitMs / 1_000);
	for (let i = 0; i < attempts; i++) {
		const res = await fetch(`${API_URL}/api/v2/keys/${userId}/bundles`, {
			headers: { Authorization: `Bearer ${authToken}` },
		});
		if (res.ok) {
			const bundles = (await res.json()) as KeyBundleMinimal[];
			if (bundles.length > 0) return;
		}
		await new Promise<void>((resolve) => setTimeout(resolve, 1_000));
	}
	throw new Error(`Key bundles for user ${userId} not available after ${maxWaitMs / 1000} s`);
}

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

/**
 * Navigate client-side (without a hard reload) by injecting and clicking an anchor.
 * SvelteKit intercepts link clicks on the document and handles them as client-side
 * navigation, keeping in-memory stores (including authStore) intact.
 *
 * Use this instead of page.goto() when the page is already authenticated and you
 * want to navigate to a new route without triggering a full page reload / auth re-init.
 */
export async function navigateClientSide(page: Page, path: string, timeout = 20_000): Promise<void> {
	await page.evaluate((href) => {
		const a = document.createElement('a');
		a.href = href;
		document.body.appendChild(a);
		a.click();
		a.remove();
	}, path);
	await page.waitForURL(`**${path}`, { timeout });
}
