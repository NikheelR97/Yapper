<script lang="ts">
	import { page } from '$app/stores';

	// Only show in Tauri
	const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI__;

	$: channelName = $page.url.pathname.includes('/channels/')
		? '#' + ($page.url.pathname.split('/').pop() ?? 'general')
		: null;

	async function minimize() {
		if (isTauri) {
			const { appWindow } = await import('@tauri-apps/api/window');
			appWindow.minimize();
		}
	}

	async function toggleMaximize() {
		if (isTauri) {
			const { appWindow } = await import('@tauri-apps/api/window');
			appWindow.toggleMaximize();
		}
	}

	async function close() {
		if (isTauri) {
			const { appWindow } = await import('@tauri-apps/api/window');
			appWindow.close();
		}
	}
</script>

{#if isTauri}
	<div class="title-bar" data-tauri-drag-region>
		<div class="title-left" data-tauri-drag-region>
			<div class="app-icon" data-tauri-drag-region></div>
			<span class="app-name" data-tauri-drag-region>Yapper</span>
			{#if channelName}
				<span class="location" data-tauri-drag-region>· {channelName}</span>
			{/if}
		</div>

		<div class="window-controls">
			<button class="wc-btn" on:click={minimize} title="Minimize" aria-label="Minimize">─</button>
			<button class="wc-btn" on:click={toggleMaximize} title="Maximize" aria-label="Maximize">□</button>
			<button class="wc-btn close" on:click={close} title="Close" aria-label="Close">✕</button>
		</div>
	</div>
{/if}

<style>
	.title-bar {
		height: 32px;
		background: #0a0a0f;
		display: flex;
		align-items: center;
		justify-content: space-between;
		-webkit-user-select: none;
		user-select: none;
		flex-shrink: 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
	}

	.title-left {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 0 12px;
		flex: 1;
	}

	.app-icon {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 35%, #c4b5fd, #7c3aed 45%, #2e1065);
		flex-shrink: 0;
	}

	.app-name {
		font-size: 13px;
		font-weight: 500;
		color: #f9fafb;
	}

	.location {
		font-size: 13px;
		color: #6b7280;
	}

	.window-controls {
		display: flex;
		align-items: stretch;
		height: 32px;
	}

	.wc-btn {
		width: 30px;
		height: 32px;
		background: none;
		border: none;
		color: #9ca3af;
		font-size: 12px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background 100ms, color 100ms;
	}

	.wc-btn:hover {
		background: rgba(255, 255, 255, 0.08);
		color: #f9fafb;
	}

	.wc-btn.close:hover {
		background: #ef4444;
		color: white;
	}
</style>
