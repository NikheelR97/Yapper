/**
 * Desktop-only native notifications.
 *
 * Guards every call with `isTauri()` — safe to import on web/mobile (all calls no-op).
 *
 * Usage:
 *   import { initDesktopNotifications, sendNativeNotification } from '$lib/desktop/notifications.js';
 *   // In (app)/+layout.svelte onMount:
 *   initDesktopNotifications();
 */

import { onWsMessage } from '$lib/stores/ws.js';
import { authStore } from '$lib/stores/auth.js';
import { get } from 'svelte/store';

// ─── Tauri guard ──────────────────────────────────────────────────────────────

function isTauri(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
}

// ─── Permission ───────────────────────────────────────────────────────────────

/** Request OS notification permission. Safe to call multiple times. */
export async function requestNotificationPermission(): Promise<void> {
    if (!isTauri()) return;
    try {
        const { isPermissionGranted, requestPermission } = await import(
            '@tauri-apps/plugin-notification'
        );
        const granted = await isPermissionGranted();
        if (!granted) {
            await requestPermission();
        }
    } catch (e) {
        console.warn('[notifications] Permission request failed:', e);
    }
}

// ─── Send ─────────────────────────────────────────────────────────────────────

/** Fire a native OS notification. No-op on web/mobile. */
export async function sendNativeNotification(
    title: string,
    body: string,
    icon?: string
): Promise<void> {
    if (!isTauri()) return;
    try {
        const { isPermissionGranted, sendNotification } = await import(
            '@tauri-apps/plugin-notification'
        );
        if (!(await isPermissionGranted())) return;
        sendNotification({ title, body, icon, sound: 'default' });
    } catch (e) {
        console.warn('[notifications] sendNotification failed:', e);
    }
}

// ─── DND check ────────────────────────────────────────────────────────────────

function isDndActive(): boolean {
    try {
        const dndEnabled = localStorage.getItem('yapper_dnd_enabled') === 'true';
        if (!dndEnabled) return false;

        const start = localStorage.getItem('yapper_dnd_start') ?? '22:00';
        const end = localStorage.getItem('yapper_dnd_end') ?? '07:00';

        const now = new Date();
        const [sh, sm] = start.split(':').map(Number);
        const [eh, em] = end.split(':').map(Number);
        const nowMins = now.getHours() * 60 + now.getMinutes();
        const startMins = sh * 60 + sm;
        const endMins = eh * 60 + em;

        // Handle overnight ranges (e.g. 22:00 → 07:00)
        if (startMins > endMins) {
            return nowMins >= startMins || nowMins < endMins;
        }
        return nowMins >= startMins && nowMins < endMins;
    } catch {
        return false;
    }
}

// ─── WS bridge ────────────────────────────────────────────────────────────────

/** Wire up WS events → native notifications. Call once in (app)/+layout.svelte. */
export function initDesktopNotifications(): void {
    if (!isTauri()) return;

    // DM notifications
    onWsMessage('dm', async (payload) => {
        const p = payload as { sender_id?: string; preview?: string };
        if (!p.sender_id) return;
        // Don't notify for own messages
        const myId = get(authStore).user?.id;
        if (p.sender_id === myId) return;
        // Only notify when window is not focused
        if (document.visibilityState === 'visible') return;
        if (isDndActive()) return;

        await sendNativeNotification('New message', p.preview ?? 'You have a new message');
    });

    // Channel @mention notifications
    onWsMessage('channel', async (payload) => {
        const p = payload as { sender_id?: string; preview?: string; mention?: boolean };
        if (!p.mention) return;
        if (!p.sender_id) return;
        const myId = get(authStore).user?.id;
        if (p.sender_id === myId) return;
        if (document.visibilityState === 'visible') return;
        if (isDndActive()) return;

        await sendNativeNotification('You were mentioned', p.preview ?? 'Someone mentioned you');
    });
}
