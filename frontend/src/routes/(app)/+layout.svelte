<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';
	import { api } from '$api/client.js';
	import type { User } from '$stores/auth.js';
	import { setAuth } from '$stores/auth.js';

	let ready = false;

	onMount(async () => {
		const state = get(authStore);

		if (!state.user) {
			// Try to refresh session via HttpOnly cookie
			try {
				const res = await api.post<{ access_token: string }>('/api/v1/auth/refresh');
				// Re-fetch current user
				const user = await api.get<User>('/api/v1/users/me');
				setAuth(user, res.access_token);
				ready = true;
			} catch {
				await goto('/login');
				return;
			}
		} else {
			ready = true;
		}
	});
</script>

{#if ready}
	<div class="app-shell">
		<slot />
	</div>
{:else}
	<div class="loading-shell">
		<div class="loader" aria-label="Loading…"></div>
	</div>
{/if}

<style>
	.app-shell {
		display: flex;
		height: 100vh;
		height: 100dvh;
		overflow: hidden;
	}

	.loading-shell {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		background: var(--color-bg-base);
	}

	.loader {
		width: 32px;
		height: 32px;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-brand);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin { to { transform: rotate(360deg); } }
</style>
