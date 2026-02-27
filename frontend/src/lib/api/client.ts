import { get } from 'svelte/store';
import { authStore } from '$stores/auth.js';

const BASE_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080';

class ApiError extends Error {
	constructor(public status: number, message: string) {
		super(message);
		this.name = 'ApiError';
	}
}

async function request<T>(
	path: string,
	options: RequestInit = {}
): Promise<T> {
	const { accessToken } = get(authStore);

	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
		...(options.headers as Record<string, string> ?? {}),
	};

	if (accessToken) {
		headers['Authorization'] = `Bearer ${accessToken}`;
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
