<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';

	export let x: number = 0;
	export let y: number = 0;
	export let items: ContextMenuItem[] = [];

	export interface ContextMenuItem {
		label: string;
		icon?: string;
		action?: () => void;
		destructive?: boolean;
		divider?: boolean; // renders a divider BEFORE this item
	}

	const dispatch = createEventDispatcher<{ close: void }>();

	function close() {
		dispatch('close');
	}

	function handleItem(item: ContextMenuItem) {
		if (item.action) item.action();
		close();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') close();
	}

	function handleOutsideClick(e: MouseEvent) {
		close();
	}

	// Clamp menu to viewport
	let menuEl: HTMLDivElement;
	let adjustedX = x;
	let adjustedY = y;

	onMount(() => {
		if (menuEl) {
			const rect = menuEl.getBoundingClientRect();
			adjustedX = x + rect.width > window.innerWidth ? x - rect.width : x;
			adjustedY = y + rect.height > window.innerHeight ? y - rect.height : y;
		}
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

<svelte:window on:click={handleOutsideClick} />

<div
	class="context-menu"
	style="left: {adjustedX}px; top: {adjustedY}px"
	bind:this={menuEl}
	role="menu"
>
	{#each items as item}
		{#if item.divider}
			<div class="divider"></div>
		{/if}
		<button
			class="menu-item"
			class:destructive={item.destructive}
			role="menuitem"
			on:click|stopPropagation={() => handleItem(item)}
		>
			{#if item.icon}
				<span class="item-icon">{item.icon}</span>
			{/if}
			<span class="item-label">{item.label}</span>
		</button>
	{/each}
</div>

<style>
	.context-menu {
		position: fixed;
		z-index: 10000;
		width: 200px;
		background: #1c1d26;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 10px;
		padding: 4px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
		animation: pop-in 120ms ease-out;
	}

	.menu-item {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		height: 36px;
		padding: 0 12px;
		background: none;
		border: none;
		border-radius: 6px;
		color: #f9fafb;
		font-size: 14px;
		font-family: inherit;
		cursor: pointer;
		text-align: left;
		transition: background 100ms, color 100ms;
	}

	.menu-item:hover {
		background: rgba(124, 58, 237, 0.1);
		color: #a78bfa;
	}

	.menu-item.destructive {
		color: #ef4444;
	}

	.menu-item.destructive:hover {
		background: rgba(239, 68, 68, 0.1);
		color: #ef4444;
	}

	.item-icon {
		font-size: 15px;
		flex-shrink: 0;
	}

	.divider {
		height: 1px;
		background: rgba(255, 255, 255, 0.06);
		margin: 4px 0;
	}

	@keyframes pop-in {
		from {
			opacity: 0;
			transform: scale(0.95);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}
</style>
