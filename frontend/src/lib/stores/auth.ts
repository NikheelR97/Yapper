import { writable } from 'svelte/store';

export interface User {
	id: string;
	username: string;
	displayName: string;
	avatarUrl: string | null;
	accountType: 'standard' | 'parent' | 'child' | 'bot';
	isPremium: boolean;
}

interface AuthState {
	user: User | null;
	accessToken: string | null;
	csrfToken: string | null;
	loading: boolean;
}

const initial: AuthState = {
	user: null,
	accessToken: null,
	csrfToken: null,
	loading: true,
};

export const authStore = writable<AuthState>(initial);

export function setAuth(user: User, accessToken: string, csrfToken?: string | null) {
	authStore.update((s) => ({
		...s,
		user,
		accessToken,
		csrfToken: csrfToken ?? s.csrfToken,
		loading: false,
	}));
}

export function clearAuth() {
	authStore.set({ user: null, accessToken: null, csrfToken: null, loading: false });
}

export function setPremiumStatus(isPremium: boolean) {
	authStore.update((s) => ({
		...s,
		user: s.user ? { ...s.user, isPremium } : null,
	}));
}
