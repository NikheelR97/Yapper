import { get } from 'svelte/store';
import { authStore } from '$stores/auth.js';

const BASE_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080';

class ApiError extends Error {
	constructor(public status: number, message: string) {
		super(message);
		this.name = 'ApiError';
	}
}

/** Read the CSRF token from the auth store (set on login/refresh response body).
 * Using the response body instead of document.cookie works cross-origin —
 * Tauri (tauri://localhost) and Capacitor can't read cookies set by http://localhost:8080. */
function getCsrfToken(): string | null {
	return get(authStore).csrfToken ?? null;
}

async function request<T>(
	path: string,
	options: RequestInit = {}
): Promise<T> {
	const { accessToken } = get(authStore);
	const method = (options.method ?? 'GET').toUpperCase();

	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
		...(options.headers as Record<string, string> ?? {}),
	};

	if (accessToken) {
		headers['Authorization'] = `Bearer ${accessToken}`;
	}

	// CSRF double-submit: echo the non-HttpOnly cookie value as a header.
	if (['POST', 'PUT', 'PATCH', 'DELETE'].includes(method)) {
		const csrf = getCsrfToken();
		if (csrf) headers['X-CSRF-Token'] = csrf;
	}

	const response = await fetch(`${BASE_URL}${path}`, {
		...options,
		headers,
		credentials: 'include', // include HttpOnly refresh cookie
	});

	if (!response.ok) {
		const body = await response.json().catch(() => ({ error: 'Unknown error' }));
		throw new ApiError(response.status, body.error ?? response.statusText);
	}

	if (response.status === 204) {
		return undefined as T;
	}

	return response.json();
}

export const api = {
	get: <T>(path: string) => request<T>(path),
	post: <T>(path: string, body?: unknown) =>
		request<T>(path, { method: 'POST', body: body ? JSON.stringify(body) : undefined }),
	patch: <T>(path: string, body?: unknown) =>
		request<T>(path, { method: 'PATCH', body: body ? JSON.stringify(body) : undefined }),
	put: <T>(path: string, body?: unknown) =>
		request<T>(path, { method: 'PUT', body: body ? JSON.stringify(body) : undefined }),
	delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};

export { ApiError };
