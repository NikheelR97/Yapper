<script lang="ts">
	import { page } from "$app/stores";
	import { isTauri as _isTauri } from "$lib/plugins/tauri-compat.js";

	// Only show in Tauri
	const isTauri = _isTauri();

	$: channelName = $page.url.pathname.includes("/channels/")
		? "#" + ($page.url.pathname.split("/").pop() ?? "general")
		: null;

	async function minimize() {
		if (isTauri) {
			const { getCurrentWindow } = await import("@tauri-apps/api/window");
			getCurrentWindow().minimize();
		}
	}

	async function toggleMaximize() {
		if (isTauri) {
			const { getCurrentWindow } = await import("@tauri-apps/api/window");
			getCurrentWindow().toggleMaximize();
		}
	}

	async function close() {
		if (isTauri) {
			const { getCurrentWindow } = await import("@tauri-apps/api/window");
			getCurrentWindow().close();
		}
	}
</script>

{#if isTauri}
	<div class="title-bar">
		<div class="title-left" data-tauri-drag-region>
			<div class="app-icon" data-tauri-drag-region></div>
			<span class="app-name" data-tauri-drag-region>Yapper</span>
			{#if channelName}
				<span class="location" data-tauri-drag-region
					>· {channelName}</span
				>
			{/if}
		</div>

		<div class="window-controls">
			<button class="wc-btn" on:click={minimize} title="Minimize" aria-label="Minimize">
				<svg width="10" height="1" viewBox="0 0 10 1" fill="none" aria-hidden="true">
					<rect width="10" height="1" fill="currentColor" />
				</svg>
			</button>
			<button class="wc-btn" on:click={toggleMaximize} title="Maximize" aria-label="Maximize">
				<svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
					<rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" stroke-width="1" fill="none" />
				</svg>
			</button>
			<button class="wc-btn close" on:click={close} title="Close" aria-label="Close">
				<svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
					<line x1="0.5" y1="0.5" x2="9.5" y2="9.5" stroke="currentColor" stroke-width="1.2" />
					<line x1="9.5" y1="0.5" x2="0.5" y2="9.5" stroke="currentColor" stroke-width="1.2" />
				</svg>
			</button>
		</div>
	</div>
{/if}

<style>
	.title-bar {
		height: 32px;
		background: transparent;
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
		background: radial-gradient(
			circle at 35% 35%,
			#c4b5fd,
			#7c3aed 45%,
			#2e1065
		);
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
		width: 46px;
		height: 32px;
		background: none;
		border: none;
		color: #9ca3af;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition:
			background 100ms,
			color 100ms;
	}

	.wc-btn:hover {
		background: rgba(255, 255, 255, 0.08);
		color: #f9fafb;
	}

	.wc-btn.close:hover {
		background: #c42b1c;
		color: white;
	}
</style>
