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
	import { toast } from "$stores/toast.js";
	import { wsStore } from "$stores/ws.js";
	import { setupKeys, replenishPreKeysIfNeeded } from "$signal/index.js";
	import { wsConnect, wsDisconnect } from "$stores/ws.js";
	import {
		registerDmHandler,
		fetchConversations,
	} from "$stores/conversations.js";
	import { registerChannelHandler, registerEmojiHandler, fetchServers } from "$stores/servers.js";
	import { registerPresenceHandler } from "$stores/presence.js";
	import { registerCanvasHandler } from "$stores/canvas.js";
	import {
		reportScreenTimeUsage,
		startYapperUsageTracker,
	} from "$plugins/screentime.js";

	// Desktop Integrations (no-op on web/mobile)
	import { initUpdater } from "$lib/desktop/updater.js";
	import { initDeepLinks } from "$lib/desktop/deepLinks.js";
	import {
		requestNotificationPermission,
		initDesktopNotifications,
	} from "$lib/desktop/notifications.js";
	import UpdateBanner from "$components/desktop/UpdateBanner.svelte";
	import TopNav from "$lib/components/nav/TopNav.svelte";
	import AppSidebar from "$lib/components/nav/AppSidebar.svelte";

	let ready = false;
	let showShortcuts = false;
	let unregisterDmHandler: (() => void) | null = null;
	let unregisterChannelHandler: (() => void) | null = null;
	let unregisterPresenceHandler: (() => void) | null = null;
	let unregisterCanvasHandler: (() => void) | null = null;
	let unregisterEmojiHandler: (() => void) | null = null;
	let stopScreenTimeTracker: (() => void) | null = null;

	onMount(async () => {
		const state = get(authStore);

		if (!state.user) {
			// Try to refresh session via HttpOnly cookie
			try {
				const res = await api.post<{ access_token: string; csrf_token: string }>(
					"/api/v1/auth/refresh",
				);
				const user = await api.get<User>("/api/v1/users/me");
				setAuth(user, res.access_token, res.csrf_token);
			} catch {
				toast.error("Session expired. Please sign in again.");
				await goto("/login");
				return;
			}
		}

		// Signal Protocol setup (idempotent — no-op if keys already exist)
		try {
			await setupKeys();
			await replenishPreKeysIfNeeded();
		} catch (e) {
			console.error("[Signal] Key setup failed:", e);
			toast.error("Encryption setup failed — try refreshing.");
		}

		// WebSocket + message handlers
		wsConnect();
		unregisterDmHandler = registerDmHandler();
		unregisterChannelHandler = registerChannelHandler();
		unregisterPresenceHandler = registerPresenceHandler();
		unregisterCanvasHandler = registerCanvasHandler();
		unregisterEmojiHandler = registerEmojiHandler();
		fetchConversations().catch(() => {});
		fetchServers().catch(() => {});

		stopScreenTimeTracker = startYapperUsageTracker();
		reportScreenTimeUsage().catch(() => {});

		// Tauri Desktop initializers
		initUpdater();
		initDeepLinks();
		requestNotificationPermission();
		initDesktopNotifications();

		ready = true;
	});

	onDestroy(() => {
		unregisterDmHandler?.();
		unregisterChannelHandler?.();
		unregisterPresenceHandler?.();
		unregisterCanvasHandler?.();
		unregisterEmojiHandler?.();
		stopScreenTimeTracker?.();
		reportScreenTimeUsage().catch(() => {});
		wsDisconnect();
	});

	function handleGlobalKeydown(e: KeyboardEvent) {
		// Ctrl+/ → keyboard shortcuts modal
		if (e.ctrlKey && e.key === "/") {
			e.preventDefault();
			showShortcuts = !showShortcuts;
		}
	}
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="layout-root">
	<TitleBar />
	<UpdateBanner />
	<TopNav />

	{#if ready && !$wsStore.connected}
		<div class="reconnecting-banner">Reconnecting…</div>
	{/if}

	{#if ready}
		<div class="app-shell">
			<AppSidebar />
			<main class="main-content">
				<slot />
			</main>
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

	.main-content {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.reconnecting-banner {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 100;
		background: #b45309;
		color: #fff;
		text-align: center;
		padding: 4px;
		font-size: 12px;
	}
</style>
