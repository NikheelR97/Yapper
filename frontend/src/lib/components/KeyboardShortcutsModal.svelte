<script lang="ts">
	import { tick } from 'svelte';

	export let open = false;
	let modalOverlay: HTMLDivElement | null = null;

	function close() {
		open = false;
	}

	$: if (open) {
		void focusModal();
	}

	async function focusModal() {
		await tick();
		modalOverlay?.focus();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) close();
	}

	function handleOverlayClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}

	function handleOverlayKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
		}
	}

	const sections = [
		{
			title: 'NAVIGATION',
			shortcuts: [
				{ keys: ['Ctrl', 'K'], desc: 'Open quick search' },
				{ keys: ['Ctrl', ','], desc: 'Open settings' },
				{ keys: ['Alt', '↑ / ↓'], desc: 'Navigate channels' },
			],
		},
		{
			title: 'MESSAGING',
			shortcuts: [
				{ keys: ['Enter'], desc: 'Send message' },
				{ keys: ['Shift', 'Enter'], desc: 'New line' },
				{ keys: ['↑'], desc: 'Edit last message (empty input)' },
				{ keys: ['Escape'], desc: 'Cancel / close modal' },
			],
		},
		{
			title: 'MEDIA',
			shortcuts: [
				{ keys: ['Ctrl', 'Y'], desc: 'Start Yap recording' },
				{ keys: ['Ctrl', 'Shift', 'V'], desc: 'Start Clip recording' },
			],
		},
		{
			title: 'OTHER',
			shortcuts: [
				{ keys: ['Ctrl', '/'], desc: 'Show this help' },
				{ keys: ['Ctrl', 'M'], desc: 'Toggle mute' },
			],
		},
	];
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
	<div
		class="modal-overlay"
		bind:this={modalOverlay}
		on:click={handleOverlayClick}
		on:keydown={handleOverlayKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Keyboard shortcuts"
		tabindex="-1"
	>
		<div class="modal-card">
			<div class="modal-header">
				<h2 class="modal-title">Keyboard Shortcuts</h2>
				<button class="close-btn" on:click={close} aria-label="Close">✕</button>
			</div>

			<div class="shortcuts-list">
				{#each sections as section}
					<div class="section">
						<div class="section-title">{section.title}</div>
						{#each section.shortcuts as shortcut}
							<div class="shortcut-row">
								<span class="shortcut-desc">{shortcut.desc}</span>
								<div class="key-combo">
									{#each shortcut.keys as key, i}
										{#if i > 0}<span class="key-sep">+</span>{/if}
										<kbd class="key">{key}</kbd>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		z-index: 9000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.modal-card {
		background: rgba(22, 23, 30, 0.95);
		border: 1px solid rgba(255, 255, 255, 0.1);
		backdrop-filter: blur(20px);
		border-radius: 16px;
		width: 100%;
		max-width: 480px;
		overflow: hidden;
		animation: fade-in 150ms ease-out;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 20px 24px 16px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.07);
	}

	.modal-title {
		font-size: 16px;
		font-weight: 700;
		color: #f9fafb;
		margin: 0;
	}

	.close-btn {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 14px;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 6px;
		transition: color 150ms, background 150ms;
	}

	.close-btn:hover {
		color: #f9fafb;
		background: rgba(255, 255, 255, 0.08);
	}

	.shortcuts-list {
		padding: 16px 24px 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
		max-height: 60vh;
		overflow-y: auto;
	}

	.section-title {
		font-size: 11px;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-bottom: 8px;
	}

	.shortcut-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		height: 40px;
	}

	.shortcut-desc {
		font-size: 14px;
		color: #d1d5db;
	}

	.key-combo {
		display: flex;
		align-items: center;
		gap: 4px;
		flex-shrink: 0;
	}

	.key {
		display: inline-block;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 4px;
		padding: 2px 8px;
		font-family: 'Courier New', monospace;
		font-size: 13px;
		font-weight: 500;
		color: #f9fafb;
	}

	.key-sep {
		font-size: 12px;
		color: #6b7280;
	}

	@keyframes fade-in {
		from {
			opacity: 0;
			transform: scale(0.96);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}
</style>
