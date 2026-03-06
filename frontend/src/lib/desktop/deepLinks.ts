/**
 * Deep link handler for Tauri desktop.
 *
 * Listens for events emitted by lib.rs `handle_deep_link()` and navigates
 * the SvelteKit app to the appropriate route using `goto()`.
 *
 * Supported deep links:
 *   yapper://dm/{conversationId}        → /dm/{conversationId}
 *   yapper://server/{serverId}          → /server/{serverId}
 *   yapper://invite/{code}             → accepts invite via API then navigates
 */

import { goto } from '$app/navigation';

// ─── Tauri guard ──────────────────────────────────────────────────────────────

function isTauri(): boolean {
    return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window);
}

// ─── Init ─────────────────────────────────────────────────────────────────────

/** Register all deep link listeners. Call once in (app)/+layout.svelte onMount. */
export async function initDeepLinks(): Promise<void> {
    if (!isTauri()) return;
    try {
        const { listen } = await import('@tauri-apps/api/event');

        // yapper://dm/{conversationId}
        await listen<string>('deep-link-dm', (event) => {
            const conversationId = event.payload?.trim();
            if (conversationId) {
                goto(`/dm/${conversationId}`);
            }
        });

        // yapper://server/{serverId}[/{channelId}]
        await listen<string>('deep-link-server', (event) => {
            const parts = event.payload?.trim().split('/') ?? [];
            const serverId = parts[0];
            const channelId = parts[1];
            if (serverId) {
                goto(channelId ? `/server/${serverId}/${channelId}` : `/server/${serverId}`);
            }
        });

        // yapper://invite/{code}
        await listen<string>('deep-link-invite', async (event) => {
            const code = event.payload?.trim();
            if (!code) return;
            // Navigate to the invite-accept route (handled by SvelteKit page)
            goto(`/invite/${code}`);
        });
    } catch (e) {
        console.warn('[deepLinks] Failed to set up deep link listeners:', e);
    }
}
