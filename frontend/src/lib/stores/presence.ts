/**
 * Presence store — caches online status per userId.
 *
 * Sources of truth (in priority order):
 *  1. WS `presence` events pushed by the server (immediate)
 *  2. REST GET /api/v1/users/:id/presence (initial fetch + fallback)
 *
 * Usage:
 *   const status = getPresence(userId);   // subscribe to a readable
 *   $status.online    → boolean
 *   $status.lastSeen  → ISO string | null
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { onWsMessage } from '$stores/ws.js';

interface PresenceState {
    online: boolean;
    away: boolean;
    lastSeen: string | null;
}

// Map of userId → writable presence state
const presenceMap = new Map<string, ReturnType<typeof writable<PresenceState>>>();
// Track which users have already been fetched via REST to avoid duplicate requests
const fetchedUsers = new Set<string>();

function getOrCreate(userId: string) {
    if (!presenceMap.has(userId)) {
        presenceMap.set(userId, writable<PresenceState>({ online: false, away: false, lastSeen: null }));
    }
    return presenceMap.get(userId)!;
}

/**
 * Returns a readable store for the given user's presence.
 * Kicks off a REST fetch on first access (deduplicated), then stays updated via WS.
 * Returns the writable's subscribe interface directly — no redundant derived() wrapper.
 */
export function getPresence(userId: string) {
    const store = getOrCreate(userId);

    // Fetch once per session — deduplicate across multiple components
    if (!fetchedUsers.has(userId)) {
        fetchedUsers.add(userId);
        fetchPresence(userId).catch(() => {/* swallow — best-effort */ });
    }

    return { subscribe: store.subscribe };
}

/** Manually refresh presence for a user via REST. */
export async function fetchPresence(userId: string): Promise<void> {
    try {
        const res = await api.get<{ online: boolean; last_seen_at: string | null }>(
            `/api/v1/users/${userId}/presence`
        );
        // REST endpoint doesn't expose away state — only WS events do
        getOrCreate(userId).update(s => ({ ...s, online: res.online, lastSeen: res.last_seen_at }));
    } catch {
        /* network error — leave cached state */
    }
}

/** Update presence directly (called from WS handler). */
export function setPresence(userId: string, online: boolean, away = false, lastSeen: string | null = null): void {
    getOrCreate(userId).set({ online, away, lastSeen });
}

// ── Real-time WS updates ────────────────────────────────────────────────────

/**
 * Register the global WS handler for presence events.
 * Call once at app init (e.g. in the app layout).
 * Returns an unsubscribe function.
 */
export function registerPresenceHandler(): () => void {
    return onWsMessage('presence', (raw) => {
        const evt = raw as { user_id: string; online: boolean; away: boolean; last_seen_at?: string };
        setPresence(evt.user_id, evt.online, evt.away ?? false, evt.last_seen_at ?? null);
    });
}
