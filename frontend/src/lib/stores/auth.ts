import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';

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

/** Store refresh token for native apps where cookies are blocked */
export function storeRefreshToken(token: string): void {
	try {
		localStorage.setItem(REFRESH_TOKEN_KEY, token);
	} catch { /* quota exceeded or unavailable */ }
}

export function getStoredRefreshToken(): string | null {
	try {
		return localStorage.getItem(REFRESH_TOKEN_KEY);
	} catch {
		return null;
	}
}

export function clearStoredRefreshToken(): void {
	try {
		localStorage.removeItem(REFRESH_TOKEN_KEY);
	} catch { /* ignore */ }
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
