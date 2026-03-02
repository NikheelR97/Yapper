<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let disabled = false;
	export let placeholder = 'Send a message…';

	let text = '';
	const dispatch = createEventDispatcher<{ send: string }>();

	function handleSubmit() {
		const trimmed = text.trim();
		if (!trimmed || disabled) return;
		dispatch('send', trimmed);
		text = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSubmit();
		}
	}
</script>

<form class="input-bar" on:submit|preventDefault={handleSubmit}>
	<textarea
		bind:value={text}
		on:keydown={handleKeydown}
		{placeholder}
		{disabled}
		rows={1}
		maxlength={4000}
		aria-label="Message"
		class="input"
	></textarea>
	<button type="submit" class="send-btn" disabled={disabled || !text.trim()} aria-label="Send">
		<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
			<line x1="22" y1="2" x2="11" y2="13"></line>
			<polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
		</svg>
	</button>
</form>

<style>
	.input-bar {
		display: flex;
		align-items: flex-end;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border);
		background: var(--color-bg-base);
	}

	.input {
		flex: 1;
		resize: none;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-primary);
		font-size: 0.9375rem;
		line-height: 1.5;
		max-height: 160px;
		overflow-y: auto;
		padding: 0.5rem 0.75rem;
		transition: border-color var(--transition-fast);
	}

	.input:focus {
		border-color: var(--color-brand);
		outline: none;
	}

	.input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.send-btn {
		flex-shrink: 0;
		width: 2.25rem;
		height: 2.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-brand);
		border: none;
		border-radius: 50%;
		color: white;
		cursor: pointer;
		transition: background var(--transition-fast), opacity var(--transition-fast);
	}

	.send-btn:hover:not(:disabled) {
		background: var(--color-brand-dark);
	}

	.send-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
