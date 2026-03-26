import { beforeEach, describe, expect, it, vi } from 'vitest';

import { clearAuth, setAuth, storeRefreshToken } from '$stores/auth.js';

import { api, ApiError } from './client.js';

const testUser = {
	id: 'user-1',
	username: 'tester',
	displayName: 'Tester',
	avatarUrl: null,
	accountType: 'standard' as const,
	isPremium: false,
};

describe('api client', () => {
	beforeEach(() => {
		clearAuth();
		localStorage.clear();
		vi.restoreAllMocks();
	});

	it('adds authorization headers for authenticated GET requests', async () => {
		const json = vi.fn().mockResolvedValue({ ok: true });
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json,
		});
		vi.stubGlobal('fetch', fetchMock);

		setAuth(testUser, 'access-token', 'csrf-token');

		await expect(api.get('/api/v2/me')).resolves.toEqual({ ok: true });
		expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/v2/me', {
			headers: {
				'Content-Type': 'application/json',
				Authorization: 'Bearer access-token',
			},
			credentials: 'include',
		});
		expect(json).toHaveBeenCalledOnce();
	});

	it('adds CSRF headers and serializes bodies for mutating requests', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json: vi.fn().mockResolvedValue({ saved: true }),
		});
		vi.stubGlobal('fetch', fetchMock);

		setAuth(testUser, 'access-token', 'csrf-token');

		await expect(api.post('/api/v2/channels', { name: 'General' })).resolves.toEqual({
			saved: true,
		});

		expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/v2/channels', {
			method: 'POST',
			body: JSON.stringify({ name: 'General' }),
			headers: {
				'Content-Type': 'application/json',
				Authorization: 'Bearer access-token',
				'X-CSRF-Token': 'csrf-token',
			},
			credentials: 'include',
		});
	});

	it('does not send X-Refresh-Token header in browser (non-native) refresh requests', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json: vi.fn().mockResolvedValue({ refreshed: true }),
		});
		vi.stubGlobal('fetch', fetchMock);

		// storeRefreshToken is a no-op in browser context (isNative() returns false),
		// so the refresh token is sent only via HttpOnly cookie (credentials: 'include').
		await storeRefreshToken('refresh-token');

		await expect(api.post('/api/v2/auth/refresh')).resolves.toEqual({ refreshed: true });

		expect(fetchMock).toHaveBeenCalledWith('http://localhost:8080/api/v2/auth/refresh', {
			method: 'POST',
			body: undefined,
			headers: {
				'Content-Type': 'application/json',
			},
			credentials: 'include',
		});
	});

	it('returns undefined for 204 responses', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			status: 204,
			json: vi.fn(),
		});
		vi.stubGlobal('fetch', fetchMock);

		await expect(api.delete('/api/v2/messages/message-1')).resolves.toBeUndefined();
	});

	it('throws ApiError using the response error message when available', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: false,
			status: 403,
			statusText: 'Forbidden',
			json: vi.fn().mockResolvedValue({ error: 'Denied' }),
		});
		vi.stubGlobal('fetch', fetchMock);

		await expect(api.get('/api/v2/admin')).rejects.toEqual(new ApiError(403, 'Denied'));
	});

	it('falls back to a generic error message when the response body is not JSON', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			statusText: 'Server Error',
			json: vi.fn().mockRejectedValue(new Error('invalid json')),
		});
		vi.stubGlobal('fetch', fetchMock);

		await expect(api.get('/api/v2/fail')).rejects.toEqual(new ApiError(500, 'Unknown error'));
	});
});
