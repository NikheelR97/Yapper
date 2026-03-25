import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { isNative } from '$lib/plugins/tauri-compat.js';

export interface User {
	id: string;
	username: string;
	displayName: string;
	avatarUrl: string | null;
	accountType: 'standard' | 'parent' | 'child' | 'bot';
	isPremium: boolean;
}

export interface AuthDevice {
	id: string;
	signalDeviceId: number;
	installationId: string | null;
	platform: 'web' | 'tauri' | 'capacitor';
	label: string;
	trustState: 'trusted' | 'pending_trust' | 'revoked';
	createdAt: string;
	lastSeenAt: string | null;
	approvedAt: string | null;
	revokedAt: string | null;
}

interface AuthState {
	user: User | null;
	device: AuthDevice | null;
	accessToken: string | null;
	csrfToken: string | null;
	loading: boolean;
}

const initial: AuthState = {
	user: null,
	device: null,
	accessToken: null,
	csrfToken: null,
	loading: true,
};

export const authStore = writable<AuthState>(initial);

export function setAuth(
	user: User,
	accessToken: string,
	csrfToken?: string | null,
	device?: AuthDevice | null
) {
	authStore.update((s) => ({
		...s,
		user,
		device: device ?? s.device,
		accessToken,
		csrfToken: csrfToken ?? s.csrfToken,
		loading: false,
	}));
}

export function clearAuth() {
	authStore.set({
		user: null,
		device: null,
		accessToken: null,
		csrfToken: null,
		loading: false,
	});
	clearStoredRefreshToken();
}

const REFRESH_TOKEN_KEY = 'yapper_refresh_token';

/** Store refresh token for native apps (Tauri/Capacitor) where cross-origin cookies
 *  are unreliable. Web browsers rely on the HttpOnly cookie instead — the token is
 *  never exposed to JavaScript in that context. */
export function storeRefreshToken(token: string): void {
	if (!isNative()) return; // web browsers use HttpOnly cookie
	try {
		localStorage.setItem(REFRESH_TOKEN_KEY, token);
	} catch { /* quota exceeded or unavailable — native storage fallback */ }
}

export function getStoredRefreshToken(): string | null {
	if (!isNative()) return null; // web browsers use HttpOnly cookie
	try {
		return localStorage.getItem(REFRESH_TOKEN_KEY);
	} catch {
		return null; // storage unavailable
	}
}

export function clearStoredRefreshToken(): void {
	try {
		localStorage.removeItem(REFRESH_TOKEN_KEY);
	} catch { /* storage unavailable — no-op */ }
}

/** Refresh the access token via the backend. Updates authStore on success.
 *  Returns the new access token, or null if the refresh failed. */
export async function refreshAccessToken(): Promise<string | null> {
	try {
		const res = await api.post<{
			access_token: string;
			csrf_token: string;
			refresh_token?: string;
			user: User;
		}>('/api/v2/auth/refresh');
		if (res.refresh_token) storeRefreshToken(res.refresh_token);
		setAuth(res.user, res.access_token, res.csrf_token);
		return res.access_token;
	} catch {
		return null;
	}
}

export function setPremiumStatus(isPremium: boolean) {
	authStore.update((s) => ({
		...s,
		user: s.user ? { ...s.user, isPremium } : null,
	}));
}
