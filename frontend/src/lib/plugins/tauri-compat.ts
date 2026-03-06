/**
 * Unified platform abstraction for Tauri (desktop) + Capacitor (mobile) + Web.
 *
 * Import from here instead of using platform-specific APIs directly,
 * so the same component code runs on all three targets.
 */

// ── Platform detection ────────────────────────────────────────────────────────

export type Platform = 'tauri' | 'capacitor' | 'web';

/** Returns the runtime this code is executing inside. */
export function platform(): Platform {
    if (typeof window === 'undefined') return 'web';
    if ('__TAURI_INTERNALS__' in window || '__TAURI__' in window) return 'tauri';
    if ('Capacitor' in window) return 'capacitor';
    return 'web';
}

export const isTauri = (): boolean => platform() === 'tauri';
export const isCapacitor = (): boolean => platform() === 'capacitor';
/** True when running inside Tauri (desktop) or Capacitor (mobile), not a plain browser. */
export const isNative = (): boolean => platform() !== 'web';

// ── Notifications ─────────────────────────────────────────────────────────────

/**
 * Request OS-level local notification permission.
 * - Tauri: uses tauri-plugin-notification
 * - Web: uses the browser Notifications API
 * - Capacitor: no-op (push notification permission is handled at FCM registration)
 *
 * Returns true if permission is granted after the call.
 */
export async function requestNotificationPermission(): Promise<boolean> {
    switch (platform()) {
        case 'tauri': {
            try {
                const { isPermissionGranted, requestPermission } = await import(
                    '@tauri-apps/plugin-notification'
                );
                if (await isPermissionGranted()) return true;
                const result = await requestPermission();
                return result === 'granted';
            } catch {
                return false;
            }
        }
        case 'web': {
            if (!('Notification' in window)) return false;
            if (Notification.permission === 'granted') return true;
            const result = await Notification.requestPermission();
            return result === 'granted';
        }
        default:
            return false;
    }
}

/**
 * Send a local (non-push) notification. Safe to call on any platform.
 * Silently no-ops if permission is not granted or the platform is unsupported.
 */
export async function sendLocalNotification(title: string, body: string): Promise<void> {
    switch (platform()) {
        case 'tauri': {
            try {
                const { isPermissionGranted, sendNotification } = await import(
                    '@tauri-apps/plugin-notification'
                );
                if (!(await isPermissionGranted())) return;
                sendNotification({ title, body, sound: 'default' });
            } catch {}
            break;
        }
        case 'web': {
            if (Notification.permission !== 'granted') return;
            new Notification(title, { body, icon: '/favicon.png' });
            break;
        }
        // Capacitor: backgrounded notifications come via FCM push; local
        // notifications would need @capacitor/local-notifications (not installed).
    }
}

// ── Badge / tray count ────────────────────────────────────────────────────────

/**
 * Update the OS-level unread badge or tray icon count.
 * - Tauri: updates the system tray tooltip to "(N)" when count > 0
 * - Capacitor / Web: no-op (badge count is set server-side via FCM payload)
 */
export async function setBadgeCount(count: number): Promise<void> {
    if (platform() !== 'tauri') return;
    try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('set_tray_badge', { count });
    } catch {}
}

// ── Window management (Tauri only) ────────────────────────────────────────────

/** Show and focus the main Tauri window. No-op on web/Capacitor. */
export async function showWindow(): Promise<void> {
    if (platform() !== 'tauri') return;
    try {
        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('show_window');
    } catch {}
}

/** Minimise the Tauri window to the taskbar/tray. No-op on web/Capacitor. */
export async function minimizeWindow(): Promise<void> {
    if (platform() !== 'tauri') return;
    try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().minimize();
    } catch {}
}

// ── Share ─────────────────────────────────────────────────────────────────────

/**
 * Open the native share sheet (mobile) or Web Share API (browser).
 * Falls back to writing `url ?? text` to the clipboard when unavailable.
 */
export async function shareContent(title: string, text: string, url?: string): Promise<void> {
    if (navigator.share) {
        try {
            await navigator.share({ title, text, url });
            return;
        } catch {
            // User cancelled — don't fall through to clipboard
            return;
        }
    }
    // Fallback: copy the most useful value to clipboard
    const fallback = url ?? text;
    try {
        await navigator.clipboard.writeText(fallback);
    } catch {}
}

// ── Clipboard ─────────────────────────────────────────────────────────────────

/** Write text to the clipboard. Works on all platforms via the Web Clipboard API. */
export async function copyToClipboard(text: string): Promise<boolean> {
    try {
        await navigator.clipboard.writeText(text);
        return true;
    } catch {
        return false;
    }
}

// ── External URL ──────────────────────────────────────────────────────────────

/**
 * Open a URL in the system/default browser.
 * Works on all platforms — on Tauri this opens outside the webview.
 */
export function openExternalUrl(url: string): void {
    window.open(url, '_blank', 'noopener,noreferrer');
}
