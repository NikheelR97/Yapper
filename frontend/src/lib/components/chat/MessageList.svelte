<script lang="ts">
	import type { Message } from '$stores/conversations.js';
	import { authStore } from '$stores/auth.js';
	import { get } from 'svelte/store';

	export let messages: Message[];

	const myId = get(authStore).user?.id ?? '';

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}
</script>

<div class="message-list" role="log" aria-live="polite" aria-label="Messages">
	{#each messages as msg (msg.id)}
		<div class="message" class:own={msg.senderId === myId}>
			{#if msg.decryptError}
				<span class="bubble error" title="Decryption failed">🔒 Unable to decrypt</span>
			{:else if msg.text === null}
				<span class="bubble loading">…</span>
			{:else}
				<span class="bubble">{msg.text}</span>
			{/if}
			<time class="timestamp" datetime={msg.createdAt}>{formatTime(msg.createdAt)}</time>
		</div>
	{/each}
</div>

<style>
	.message-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 1rem;
		overflow-y: auto;
		flex: 1;
	}

	.message {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.125rem;
	}

	.message.own {
		align-items: flex-end;
	}

	.bubble {
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: 1rem 1rem 1rem 0.25rem;
		color: var(--color-text-primary);
		font-size: 0.9375rem;
		line-height: 1.45;
		max-width: 70%;
		padding: 0.5rem 0.875rem;
		word-break: break-word;
	}

	.message.own .bubble {
		background: var(--color-brand);
		border-color: transparent;
		border-radius: 1rem 1rem 0.25rem 1rem;
		color: white;
	}

	.bubble.error {
		background: rgba(239, 68, 68, 0.1);
		border-color: rgba(239, 68, 68, 0.3);
		color: #fca5a5;
		font-size: 0.8125rem;
	}

	.bubble.loading {
		color: var(--color-text-muted);
	}

	.timestamp {
		color: var(--color-text-muted);
		font-size: 0.6875rem;
		padding: 0 0.25rem;
	}
</style>
