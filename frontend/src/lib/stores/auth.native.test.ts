import { beforeEach, describe, expect, it, vi } from 'vitest';

const platformState = vi.hoisted(() => ({
	platform: 'tauri' as 'tauri' | 'capacitor',
}));

const vaultState = vi.hoisted(() => {
	const state = {
		desktopVaultUnlocked: true,
		vaultRecords: new Map<string, unknown>(),
		saveDesktopSignalVaultRecord: vi.fn(async (scopeKey: string, value: unknown) => {
			state.vaultRecords.set(scopeKey, value);
		}),
		loadDesktopSignalVaultRecord: vi.fn(async (scopeKey: string) => {
			return (
				(state.vaultRecords.get(scopeKey) as { token: string } | null | undefined) ?? null
			);
		}),
		clearDesktopSignalVaultRecord: vi.fn(async (scopeKey: string) => {
			state.vaultRecords.delete(scopeKey);
		}),
	};
	return state;
});

const mobileSecureStoreState = vi.hoisted(() => {
	const state = {
		records: new Map<string, string>(),
		getMobileSecureItem: vi.fn(async (key: string) => state.records.get(key) ?? null),
		setMobileSecureItem: vi.fn(async (key: string, value: string) => {
			state.records.set(key, value);
		}),
		removeMobileSecureItem: vi.fn(async (key: string) => {
			state.records.delete(key);
		}),
	};
	return state;
});

vi.mock('$lib/plugins/tauri-compat.js', () => ({
	isNative: () => true,
	isTauri: () => platformState.platform === 'tauri',
	isCapacitor: () => platformState.platform === 'capacitor',
}));

vi.mock('$lib/desktop/vault.js', () => ({
	clearDesktopSignalVaultRecord: vaultState.clearDesktopSignalVaultRecord,
	desktopVaultSupported: () => platformState.platform === 'tauri',
	isDesktopVaultUnlocked: () => vaultState.desktopVaultUnlocked,
	loadDesktopSignalVaultRecord: vaultState.loadDesktopSignalVaultRecord,
	saveDesktopSignalVaultRecord: vaultState.saveDesktopSignalVaultRecord,
}));

vi.mock('$lib/mobile/secure-store.js', () => ({
	getMobileSecureItem: mobileSecureStoreState.getMobileSecureItem,
	setMobileSecureItem: mobileSecureStoreState.setMobileSecureItem,
	removeMobileSecureItem: mobileSecureStoreState.removeMobileSecureItem,
}));

async function loadAuthModule() {
	return import('./auth.js');
}

describe('native refresh token storage', () => {
	beforeEach(() => {
		platformState.platform = 'tauri';
		vaultState.desktopVaultUnlocked = true;
		vaultState.vaultRecords.clear();
		mobileSecureStoreState.records.clear();
		localStorage.clear();
		vaultState.saveDesktopSignalVaultRecord.mockClear();
		vaultState.loadDesktopSignalVaultRecord.mockClear();
		vaultState.clearDesktopSignalVaultRecord.mockClear();
		mobileSecureStoreState.getMobileSecureItem.mockClear();
		mobileSecureStoreState.setMobileSecureItem.mockClear();
		mobileSecureStoreState.removeMobileSecureItem.mockClear();
		vi.resetModules();
	});

	it('stores tauri refresh tokens in secure storage instead of localStorage', async () => {
		const { clearStoredRefreshToken, getStoredRefreshToken, storeRefreshToken } =
			await loadAuthModule();
		clearStoredRefreshToken();

		await storeRefreshToken('refresh-token');

		expect(localStorage.getItem('yapper_refresh_token')).toBeNull();
		expect(vaultState.saveDesktopSignalVaultRecord).toHaveBeenCalledWith('auth:refresh-token', {
			token: 'refresh-token',
		});
		await expect(getStoredRefreshToken()).resolves.toBe('refresh-token');
	});

	it('migrates legacy tauri refresh tokens out of localStorage on first read', async () => {
		const { clearStoredRefreshToken, getStoredRefreshToken } = await loadAuthModule();
		clearStoredRefreshToken();
		localStorage.setItem('yapper_refresh_token', 'legacy-refresh-token');

		await expect(getStoredRefreshToken()).resolves.toBe('legacy-refresh-token');

		expect(localStorage.getItem('yapper_refresh_token')).toBeNull();
		expect(vaultState.saveDesktopSignalVaultRecord).toHaveBeenCalledWith('auth:refresh-token', {
			token: 'legacy-refresh-token',
		});
	});

	it('holds tauri tokens in memory until the desktop vault is unlocked', async () => {
		const { clearStoredRefreshToken, storeRefreshToken, syncNativeRefreshTokenToSecureStorage } =
			await loadAuthModule();
		clearStoredRefreshToken();
		vaultState.desktopVaultUnlocked = false;

		await storeRefreshToken('pending-refresh-token');

		expect(vaultState.saveDesktopSignalVaultRecord).not.toHaveBeenCalled();
		vaultState.desktopVaultUnlocked = true;
		await syncNativeRefreshTokenToSecureStorage();

		expect(vaultState.saveDesktopSignalVaultRecord).toHaveBeenCalledWith('auth:refresh-token', {
			token: 'pending-refresh-token',
		});
	});

	it('persists capacitor refresh tokens across module reloads', async () => {
		platformState.platform = 'capacitor';
		const first = await loadAuthModule();
		first.clearStoredRefreshToken();

		await first.storeRefreshToken('capacitor-refresh-token');

		expect(localStorage.getItem('yapper_refresh_token')).toBeNull();
		expect(mobileSecureStoreState.setMobileSecureItem).toHaveBeenCalledWith(
			'auth:refresh-token',
			'capacitor-refresh-token',
		);
		expect(vaultState.saveDesktopSignalVaultRecord).not.toHaveBeenCalled();

		vi.resetModules();
		const second = await loadAuthModule();
		await expect(second.getStoredRefreshToken()).resolves.toBe('capacitor-refresh-token');
	});

	it('migrates legacy capacitor refresh tokens into secure storage on first read', async () => {
		platformState.platform = 'capacitor';
		const first = await loadAuthModule();
		first.clearStoredRefreshToken();
		localStorage.setItem('yapper_refresh_token', 'legacy-capacitor-token');

		await expect(first.getStoredRefreshToken()).resolves.toBe('legacy-capacitor-token');

		expect(localStorage.getItem('yapper_refresh_token')).toBeNull();
		expect(mobileSecureStoreState.setMobileSecureItem).toHaveBeenCalledWith(
			'auth:refresh-token',
			'legacy-capacitor-token',
		);
	});

	it('returns a warning instead of throwing when capacitor secure storage fails', async () => {
		platformState.platform = 'capacitor';
		mobileSecureStoreState.setMobileSecureItem.mockRejectedValueOnce(
			new Error('secure storage unavailable'),
		);
		const first = await loadAuthModule();
		first.clearStoredRefreshToken();

		await expect(first.storeRefreshToken('session-refresh-token')).resolves.toMatch(
			/secure token storage is unavailable/i,
		);
		await expect(first.getStoredRefreshToken()).resolves.toBe('session-refresh-token');
	});
});
