<script lang="ts">
	import { onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { onWsMessage } from '$stores/ws.js';
	import { authStore } from '$stores/auth.js';

	export let channelId: string;

	const myId = get(authStore).user?.id ?? '';

	// Set of user IDs currently typing (excluding self)
	let typingSet = new Set<string>();
	let count = 0;

	const unsubStart = onWsMessage('typing', (payload) => {
		const p = payload as { channel_id: string; user_id: string };
		if (p.channel_id !== channelId || p.user_id === myId) return;
		typingSet.add(p.user_id);
		count = typingSet.size;
	});

	const unsubStop = onWsMessage('typing_stop', (payload) => {
		const p = payload as { channel_id: string; user_id: string };
		if (p.channel_id !== channelId) return;
		typingSet.delete(p.user_id);
		count = typingSet.size;
	});

	onDestroy(() => {
		unsubStart();
		unsubStop();
	});
</script>

{#if count > 0}
	<div class="typing" aria-live="polite" aria-label="{count === 1 ? 'Someone is' : `${count} people are`} typing">
		<span class="dots" aria-hidden="true">
			<span></span><span></span><span></span>
		</span>
		<span class="label">
			{count === 1 ? 'Someone is typing' : `${count} people are typing`}
		</span>
	</div>
{/if}

<style>
	.typing {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.25rem 1rem 0.375rem;
		min-height: 1.5rem;
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.dots {
		display: flex;
		gap: 3px;
		align-items: center;
	}

	.dots span {
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--color-text-muted);
		animation: typing-bob 1.2s ease-in-out infinite;
	}

	.dots span:nth-child(2) { animation-delay: 0.2s; }
	.dots span:nth-child(3) { animation-delay: 0.4s; }

	@keyframes typing-bob {
		0%, 60%, 100% { transform: translateY(0); }
		30%           { transform: translateY(-4px); }
	}

	@media (prefers-reduced-motion: reduce) {
		.dots span {
			animation: none;
		}
	}
</style>
