<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let channelName = '';
	export let showCanvas = false;
	export let onlineCount: number | null = null;

	const dispatch = createEventDispatcher<{ toggleCanvas: void; search: void }>();
</script>

<header class="channel-header">
	<!-- Left: channel name -->
	<div class="channel-name-group">
		<span class="hash">#</span>
		<span class="channel-name">{channelName || '…'}</span>
	</div>

	<!-- Online badge -->
	{#if onlineCount !== null}
		<div class="online-badge" aria-label="{onlineCount.toLocaleString()} online">
			<span class="dot" aria-hidden="true">●</span>
			{onlineCount.toLocaleString()} online
		</div>
	{/if}

	<div class="actions">
		<!-- Search -->
		<button
			class="icon-btn"
			on:click={() => dispatch('search')}
			aria-label="Search messages"
			title="Search messages"
		>
			<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
			</svg>
		</button>

		<!-- Canvas toggle -->
		<button
			class="icon-btn"
			class:active={showCanvas}
			on:click={() => dispatch('toggleCanvas')}
			title={showCanvas ? 'Hide Canvas' : 'Show Canvas'}
			aria-pressed={showCanvas}
			aria-label={showCanvas ? 'Hide Canvas' : 'Show Canvas'}
		>
			<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<rect x="3" y="3" width="7" height="7"/>
				<rect x="14" y="3" width="7" height="7"/>
				<rect x="14" y="14" width="7" height="7"/>
				<rect x="3" y="14" width="7" height="7"/>
			</svg>
		</button>

		<!-- E2EE badge -->
		<span class="e2ee-badge" title="End-to-end encrypted">
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
				<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
			</svg>
			E2EE
		</span>
	</div>
</header>

<style>
	.channel-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		height: 48px;
		padding: 0 1rem;
		background: var(--color-bg-elevated, #0f1117);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
		flex-shrink: 0;
	}

	.channel-name-group {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		flex: 1;
		min-width: 0;
	}

	.hash {
		font-size: 1.125rem;
		font-weight: 700;
		color: #6b7280;
		flex-shrink: 0;
		line-height: 1;
	}

	.channel-name {
		font-size: 1rem;
		font-weight: 700;
		color: #f9fafb;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.online-badge {
		display: flex;
		align-items: center;
		gap: 0.3125rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 9999px;
		padding: 0.1875rem 0.625rem;
		font-size: 0.75rem;
		color: #9ca3af;
		flex-shrink: 0;
		white-space: nowrap;
	}

	.dot {
		color: #22c55e;
		font-size: 0.5rem;
		line-height: 1;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		flex-shrink: 0;
	}

	.icon-btn {
		background: none;
		border: 1px solid transparent;
		border-radius: 0.375rem;
		padding: 0.3rem;
		color: var(--color-text-muted);
		cursor: pointer;
		display: flex;
		align-items: center;
		transition: color 0.15s, border-color 0.15s;
	}
	.icon-btn:hover {
		color: var(--color-text-primary);
		border-color: var(--color-border);
	}
	.icon-btn.active {
		color: var(--color-brand-light, #a78bfa);
		border-color: var(--color-brand);
	}

	.e2ee-badge {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		padding: 0 0.25rem;
	}
</style>
