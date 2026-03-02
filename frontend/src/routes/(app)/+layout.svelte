<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { authStore } from '$stores/auth.js';
	import { api } from '$api/client.js';
	import type { User } from '$stores/auth.js';
	import { setAuth } from '$stores/auth.js';
	import { setupKeys, replenishPreKeysIfNeeded } from '$signal/index.js';
	import { wsConnect, wsDisconnect } from '$stores/ws.js';
	import { registerDmHandler, fetchConversations } from '$stores/conversations.js';
	import { registerChannelHandler, fetchServers } from '$stores/servers.js';

	let ready = false;
	let unregisterDmHandler: (() => void) | null = null;
	let unregisterChannelHandler: (() => void) | null = null;

	onMount(async () => {
		const state = get(authStore);

		if (!state.user) {
			// Try to refresh session via HttpOnly cookie
			try {
				const res = await api.post<{ access_token: string }>('/api/v1/auth/refresh');
				const user = await api.get<User>('/api/v1/users/me');
				setAuth(user, res.access_token);
			} catch {
				await goto('/login');
				return;
			}
		}

		// Signal Protocol setup (idempotent — no-op if keys already exist)
		try {
			await setupKeys();
			await replenishPreKeysIfNeeded();
		} catch (e) {
			console.warn('[Signal] Key setup failed:', e);
		}

		// WebSocket + message handlers
		wsConnect();
		unregisterDmHandler = registerDmHandler();
		unregisterChannelHandler = registerChannelHandler();
		fetchConversations().catch(() => {});
		fetchServers().catch(() => {});

		ready = true;
	});

	onDestroy(() => {
		unregisterDmHandler?.();
		unregisterChannelHandler?.();
		wsDisconnect();
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
