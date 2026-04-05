import { writable } from 'svelte/store';
import { api } from '$api/client.js';
import { isCapacitor, isNative, isTauri } from '$lib/plugins/tauri-compat.js';
import {
	clearDesktopSignalVaultRecord,
	desktopVaultSupported,
	isDesktopVaultUnlocked,
	loadDesktopSignalVaultRecord,
	saveDesktopSignalVaultRecord,
} from '$lib/desktop/vault.js';
import {
	getMobileSecureItem,
	removeMobileSecureItem,
	setMobileSecureItem,
} from '$lib/mobile/secure-store.js';

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

interface StoredRefreshTokenRecord {
	token: string;
}

const initial: AuthState = {
	user: null,
	device: null,
	accessToken: null,
	csrfToken: null,
	loading: true,
};

const REFRESH_TOKEN_KEY = 'yapper_refresh_token';
const DESKTOP_REFRESH_TOKEN_SCOPE = 'auth:refresh-token';
const NATIVE_REFRESH_TOKEN_WARNING =
	'Signed in, but secure token storage is unavailable on this device. You may need to sign in again after restarting the app.';

export const authStore = writable<AuthState>(initial);

type SessionResetter = () => void;

const sessionResetters = new Set<SessionResetter>();
let nativeRefreshTokenMemory: string | null = null;

function readLegacyRefreshToken(): string | null {
	if (!isNative()) return null;
	try {
		return localStorage.getItem(REFRESH_TOKEN_KEY);
	} catch {
		return null;
	}
}

function clearLegacyRefreshToken(): void {
	try {
		localStorage.removeItem(REFRESH_TOKEN_KEY);
	} catch {
		// Ignore unavailable local storage while clearing other stores.
	}
}

async function persistDesktopRefreshToken(token: string): Promise<void> {
	if (!desktopVaultSupported() || !isDesktopVaultUnlocked()) {
		return;
	}

	await saveDesktopSignalVaultRecord(DESKTOP_REFRESH_TOKEN_SCOPE, { token });
}

async function loadDesktopRefreshToken(): Promise<string | null> {
	if (!desktopVaultSupported() || !isDesktopVaultUnlocked()) {
		return null;
	}

	const record =
		await loadDesktopSignalVaultRecord<StoredRefreshTokenRecord>(
			DESKTOP_REFRESH_TOKEN_SCOPE,
		);
	return record?.token ?? null;
}

async function persistCapacitorRefreshToken(token: string): Promise<void> {
	if (!isCapacitor()) {
		return;
	}

	await setMobileSecureItem(DESKTOP_REFRESH_TOKEN_SCOPE, token);
}

async function loadCapacitorRefreshToken(): Promise<string | null> {
	if (!isCapacitor()) {
		return null;
	}

	return getMobileSecureItem(DESKTOP_REFRESH_TOKEN_SCOPE);
}

async function clearCapacitorRefreshToken(): Promise<void> {
	if (!isCapacitor()) {
		return;
	}

	await removeMobileSecureItem(DESKTOP_REFRESH_TOKEN_SCOPE);
}

export function registerSessionResetter(resetter: SessionResetter): () => void {
	sessionResetters.add(resetter);
	return () => sessionResetters.delete(resetter);
}

export function resetAppState(): void {
	for (const resetter of sessionResetters) {
		try {
			resetter();
		} catch (err) {
			console.error('[auth] session resetter failed:', err);
		}
	}
}

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
	resetAppState();
	authStore.set({
		user: null,
		device: null,
		accessToken: null,
		csrfToken: null,
		loading: false,
	});
	clearStoredRefreshToken();
}

/** Store refresh token for native apps (Tauri/Capacitor) where cross-origin cookies
 *  are unreliable. Web browsers rely on the HttpOnly cookie instead - the token is
 *  never exposed to JavaScript in that context. */
export async function storeRefreshToken(token: string): Promise<string | null> {
	if (!isNative()) return null; // web browsers use HttpOnly cookie
	nativeRefreshTokenMemory = token;
	try {
		if (isTauri()) {
			clearLegacyRefreshToken();
		} else if (isCapacitor()) {
			clearLegacyRefreshToken();
			await persistCapacitorRefreshToken(token);
		}
		await persistDesktopRefreshToken(token);
		return null;
	} catch (err) {
		console.error('[auth] failed to persist native refresh token securely:', err);
		return NATIVE_REFRESH_TOKEN_WARNING;
	}
}

export async function syncNativeRefreshTokenToSecureStorage(): Promise<void> {
	if (!nativeRefreshTokenMemory) {
		const persistedToken = isCapacitor()
			? await loadCapacitorRefreshToken()
			: readLegacyRefreshToken();
		if (persistedToken) {
			nativeRefreshTokenMemory = persistedToken;
			if (isTauri()) {
				clearLegacyRefreshToken();
			}
		}
	}

	if (!nativeRefreshTokenMemory) {
		return;
	}

	if (isCapacitor()) {
		await persistCapacitorRefreshToken(nativeRefreshTokenMemory);
		return;
	}

	try {
		await persistDesktopRefreshToken(nativeRefreshTokenMemory);
	} catch (err) {
		console.error('[auth] failed to sync refresh token into secure storage:', err);
	}
}

export async function getStoredRefreshToken(): Promise<string | null> {
	if (!isNative()) return null; // web browsers use HttpOnly cookie

	if (nativeRefreshTokenMemory) {
		return nativeRefreshTokenMemory;
	}

	const persistedToken = isCapacitor()
		? await loadCapacitorRefreshToken()
		: await loadDesktopRefreshToken();
	if (persistedToken) {
		nativeRefreshTokenMemory = persistedToken;
		return nativeRefreshTokenMemory;
	}

	const legacyToken = readLegacyRefreshToken();
	if (legacyToken) {
		nativeRefreshTokenMemory = legacyToken;
		clearLegacyRefreshToken();
		await syncNativeRefreshTokenToSecureStorage();
		return nativeRefreshTokenMemory;
	}

	try {
		if (isTauri()) {
			nativeRefreshTokenMemory = await loadDesktopRefreshToken();
			return nativeRefreshTokenMemory;
		}
		if (isCapacitor()) {
			nativeRefreshTokenMemory = await loadCapacitorRefreshToken();
			return nativeRefreshTokenMemory;
		}
		return null;
	} catch (err) {
		console.warn('[auth] failed to load native refresh token:', err);
		return null;
	}
}

export function clearStoredRefreshToken(): void {
	nativeRefreshTokenMemory = null;
	clearLegacyRefreshToken();
	if (isCapacitor()) {
		void clearCapacitorRefreshToken().catch((err) => {
			console.warn('[auth] failed to clear native refresh token from mobile secure storage:', err);
		});
	}
	if (!desktopVaultSupported() || !isDesktopVaultUnlocked()) {
		return;
	}
	void clearDesktopSignalVaultRecord(DESKTOP_REFRESH_TOKEN_SCOPE).catch((err) => {
		console.warn('[auth] failed to clear native refresh token from secure storage:', err);
	});
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
		if (res.refresh_token) await storeRefreshToken(res.refresh_token);
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
