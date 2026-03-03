<script lang="ts">
	import { goto } from "$app/navigation";
	import { onMount, onDestroy } from "svelte";
	import { get } from "svelte/store";
	import Toast from "$components/Toast.svelte";
	import TitleBar from "$components/TitleBar.svelte";
	import AppLoadingScreen from "$components/AppLoadingScreen.svelte";
	import KeyboardShortcutsModal from "$components/KeyboardShortcutsModal.svelte";
	import { authStore } from "$stores/auth.js";
	import { api } from "$api/client.js";
	import type { User } from "$stores/auth.js";
	import { setAuth } from "$stores/auth.js";
	import { setupKeys, replenishPreKeysIfNeeded } from "$signal/index.js";
	import { wsConnect, wsDisconnect } from "$stores/ws.js";
	import {
		registerDmHandler,
		fetchConversations,
	} from "$stores/conversations.js";
	import { registerChannelHandler, fetchServers } from "$stores/servers.js";
	import { registerPresenceHandler } from "$stores/presence.js";
	import { registerCanvasHandler } from "$stores/canvas.js";

	let ready = false;
	let showShortcuts = false;
	let unregisterDmHandler: (() => void) | null = null;
	let unregisterChannelHandler: (() => void) | null = null;
	let unregisterPresenceHandler: (() => void) | null = null;
	let unregisterCanvasHandler: (() => void) | null = null;

	onMount(async () => {
		const state = get(authStore);

		if (!state.user) {
			// Try to refresh session via HttpOnly cookie
			try {
				const res = await api.post<{ access_token: string }>(
					"/api/v1/auth/refresh",
				);
				const user = await api.get<User>("/api/v1/users/me");
				setAuth(user, res.access_token);
			} catch {
				await goto("/login");
				return;
			}
		}

		// Signal Protocol setup (idempotent — no-op if keys already exist)
		try {
			await setupKeys();
			await replenishPreKeysIfNeeded();
		} catch (e) {
			console.warn("[Signal] Key setup failed:", e);
		}

		// WebSocket + message handlers
		wsConnect();
		unregisterDmHandler = registerDmHandler();
		unregisterChannelHandler = registerChannelHandler();
		unregisterPresenceHandler = registerPresenceHandler();
		unregisterCanvasHandler = registerCanvasHandler();
		fetchConversations().catch(() => {});
		fetchServers().catch(() => {});

		ready = true;
	});

	onDestroy(() => {
		unregisterDmHandler?.();
		unregisterChannelHandler?.();
		unregisterPresenceHandler?.();
		unregisterCanvasHandler?.();
		wsDisconnect();
	});

	function handleGlobalKeydown(e: KeyboardEvent) {
		// Ctrl+/ → keyboard shortcuts modal
		if (e.ctrlKey && e.key === '/') {
			e.preventDefault();
			showShortcuts = !showShortcuts;
		}
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="layout-root">
	<TitleBar />

	{#if ready}
		<div class="app-shell">
			<slot />
		</div>
	{:else}
		<AppLoadingScreen />
	{/if}
</div>

<Toast />
<KeyboardShortcutsModal bind:open={showShortcuts} />

<style>
	.layout-root {
		display: flex;
		flex-direction: column;
		height: 100vh;
		height: 100dvh;
		overflow: hidden;
	}

	.app-shell {
		display: flex;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
</style>
