import { beforeEach, describe, expect, it, vi } from 'vitest';

const vaultState = vi.hoisted(() => ({
	desktopVaultUnlocked: true,
	vaultRecords: new Map<string, unknown>(),
}));

vi.mock('$lib/plugins/tauri-compat.js', () => ({
	isNative: () => true,
	isTauri: () => true,
	isCapacitor: () => false,
}));

vi.mock('$lib/desktop/vault.js', () => ({
	clearDesktopSignalVaultRecord: vi.fn(async (scopeKey: string) => {
		vaultState.vaultRecords.delete(scopeKey);
	}),
	desktopVaultSupported: () => true,
	isDesktopVaultUnlocked: () => vaultState.desktopVaultUnlocked,
	loadDesktopSignalVaultRecord: vi.fn(async (scopeKey: string) => {
		return (vaultState.vaultRecords.get(scopeKey) as { token: string } | null | undefined) ?? null;
	}),
	saveDesktopSignalVaultRecord: vi.fn(async (scopeKey: string, value: unknown) => {
		vaultState.vaultRecords.set(scopeKey, value);
	}),
}));

import { clearAuth, storeRefreshToken } from '$stores/auth.js';
import { api } from './client.js';

describe('api client native refresh token handling', () => {
	beforeEach(() => {
		vaultState.desktopVaultUnlocked = true;
		vaultState.vaultRecords.clear();
		clearAuth();
		localStorage.clear();
		vi.restoreAllMocks();
	});

	it('sends X-Refresh-Token on native refresh requests using secure storage', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json: vi.fn().mockResolvedValue({ refreshed: true }),
		});
		vi.stubGlobal('fetch', fetchMock);

		await storeRefreshToken('native-refresh-token');
		await expect(api.post('/api/v2/auth/refresh')).resolves.toEqual({ refreshed: true });

		expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/v2/auth/refresh', {
			method: 'POST',
			body: undefined,
			headers: {
				'Content-Type': 'application/json',
				'X-Refresh-Token': 'native-refresh-token',
			},
			credentials: 'include',
		});
	});
});
