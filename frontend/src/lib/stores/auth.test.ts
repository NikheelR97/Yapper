import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { authStore, setAuth, clearAuth, registerSessionResetter, resetAppState } from './auth.js';
import type { AuthDevice, User } from './auth.js';

const testUser: User = {
	id: 'test-uuid-1234',
	username: 'testuser',
	displayName: 'Test User',
	avatarUrl: null,
	accountType: 'standard',
	isPremium: false,
};

const testDevice: AuthDevice = {
	id: 'device-uuid-1234',
	signalDeviceId: 7,
	installationId: 'install-123',
	platform: 'web',
	label: 'Web Browser',
	trustState: 'trusted',
	createdAt: new Date().toISOString(),
	lastSeenAt: null,
	approvedAt: new Date().toISOString(),
	revokedAt: null,
};

describe('authStore', () => {
	beforeEach(() => {
		clearAuth();
	});

	it('starts with no user', () => {
		const state = get(authStore);
		expect(state.user).toBeNull();
		expect(state.device).toBeNull();
		expect(state.accessToken).toBeNull();
	});

	it('setAuth stores user and token', () => {
		setAuth(testUser, 'test-token');
		const state = get(authStore);
		expect(state.user).toEqual(testUser);
		expect(state.device).toBeNull();
		expect(state.accessToken).toBe('test-token');
		expect(state.loading).toBe(false);
	});

	it('setAuth stores the active device when provided', () => {
		setAuth(testUser, 'test-token', 'csrf-token', testDevice);
		const state = get(authStore);
		expect(state.user).toEqual(testUser);
		expect(state.device).toEqual(testDevice);
		expect(state.csrfToken).toBe('csrf-token');
	});

	it('clearAuth removes user and token', () => {
		setAuth(testUser, 'test-token', 'csrf-token', testDevice);
		clearAuth();
		const state = get(authStore);
		expect(state.user).toBeNull();
		expect(state.device).toBeNull();
		expect(state.accessToken).toBeNull();
	});

	it('clearAuth runs registered session resetters before clearing auth state', () => {
		const resetter = vi.fn();
		const unregister = registerSessionResetter(resetter);

		try {
			setAuth(testUser, 'test-token', 'csrf-token', testDevice);
			clearAuth();
			expect(resetter).toHaveBeenCalledOnce();
			expect(get(authStore).user).toBeNull();
		} finally {
			unregister();
		}
	});

	it('resetAppState can be triggered directly for session-scoped caches', () => {
		const resetter = vi.fn();
		const unregister = registerSessionResetter(resetter);

		try {
			resetAppState();
			expect(resetter).toHaveBeenCalledOnce();
		} finally {
			unregister();
		}
	});
});
