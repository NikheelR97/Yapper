import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { authStore, setAuth, clearAuth } from './auth.js';
import type { User } from './auth.js';

const testUser: User = {
	id: 'test-uuid-1234',
	username: 'testuser',
	displayName: 'Test User',
	avatarUrl: null,
	accountType: 'standard',
	isPremium: false,
};

describe('authStore', () => {
	beforeEach(() => {
		clearAuth();
	});

	it('starts with no user', () => {
		const state = get(authStore);
		expect(state.user).toBeNull();
		expect(state.accessToken).toBeNull();
	});

	it('setAuth stores user and token', () => {
		setAuth(testUser, 'test-token');
		const state = get(authStore);
		expect(state.user).toEqual(testUser);
		expect(state.accessToken).toBe('test-token');
		expect(state.loading).toBe(false);
	});

	it('clearAuth removes user and token', () => {
		setAuth(testUser, 'test-token');
		clearAuth();
		const state = get(authStore);
		expect(state.user).toBeNull();
		expect(state.accessToken).toBeNull();
	});
});
