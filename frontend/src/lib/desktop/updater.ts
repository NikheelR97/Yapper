/**
 * Auto-updater bridge for Tauri desktop.
 *
 * - Silently checks for an update in the background (called from layout on mount).
 * - Exposes an `updateAvailable` store that the UpdateBanner component subscribes to.
 * - `installUpdate()` downloads and installs, then relaunches.
 */

import { writable } from 'svelte/store';

// ─── Store ────────────────────────────────────────────────────────────────────

/** Non-null when a new version is available. */
export const updateAvailable = writable<string | null>(null);

// ─── Tauri guard ──────────────────────────────────────────────────────────────

function isTauri(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
}

// ─── Check ────────────────────────────────────────────────────────────────────

/**
 * Kick off a silent background update check.
 * If an update is found, the Rust side emits "update-available" which we listen for here.
 * The actual check runs in Rust (see lib.rs `check_for_update`); this just sets up the listener.
 */
export async function initUpdater(): Promise<void> {
    if (!isTauri()) return;
    try {
        const { listen } = await import('@tauri-apps/api/event');
        await listen<string>('update-available', (event) => {
            updateAvailable.set(event.payload);
        });
    } catch (e) {
        console.warn('[updater] Failed to set up update listener:', e);
    }
}

/** Download and install the pending update, then relaunch. */
export async function installUpdate(): Promise<void> {
    if (!isTauri()) return;
    try {
        const { check } = await import('@tauri-apps/plugin-updater');
        const { relaunch } = await import('@tauri-apps/plugin-process');
        const update = await check();
        if (!update) return;

        await update.downloadAndInstall((progress) => {
            console.info('[updater] Download progress:', progress);
        });
        await relaunch();
    } catch (e) {
        console.error('[updater] Install failed:', e);
    }
}
