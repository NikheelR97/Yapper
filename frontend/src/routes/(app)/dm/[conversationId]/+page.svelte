<script lang="ts">
	import { page } from '$app/stores';
	import { onMount, afterUpdate } from 'svelte';
	import { get } from 'svelte/store';
	import { conversationsStore, getMessageStore, loadMessages, sendMessage } from '$stores/conversations.js';
	import MessageList from '$lib/components/chat/MessageList.svelte';
	import MessageInput from '$lib/components/chat/MessageInput.svelte';

	$: conversationId = $page.params.conversationId ?? '';

	// Look up conversation metadata from the store
	$: conversation = $conversationsStore.conversations.find((c) => c.id === conversationId);
	$: messages$ = getMessageStore(conversationId);

	let sending = false;
	let loadError = false;
	let listEl: HTMLDivElement;

	onMount(async () => {
		if (!conversation) return;
		try {
			await loadMessages(conversationId, conversation.peerId);
		} catch {
			loadError = true;
		}
	});

	// Scroll to bottom whenever messages change
	afterUpdate(() => {
		if (listEl) {
			listEl.scrollTop = listEl.scrollHeight;
		}
	});

	async function handleSend(e: CustomEvent<string>) {
		if (!conversation) return;
		sending = true;
		try {
			await sendMessage(conversationId, conversation.peerId, e.detail);
		} finally {
			sending = false;
		}
	}
</script>

<svelte:head>
	<title>{conversation ? conversation.peerDisplayName || conversation.peerUsername : 'Direct Message'} — Yapper</title>
</svelte:head>

<div class="dm-page">
	<!-- Header -->
	<header class="dm-header">
		<div class="peer-info">
			{#if conversation?.peerAvatarUrl}
				<img
					src={conversation.peerAvatarUrl}
					alt={conversation.peerUsername}
					class="avatar"
					width="32"
					height="32"
				/>
			{:else}
				<div class="avatar-placeholder" aria-hidden="true">
					{(conversation?.peerDisplayName || conversation?.peerUsername || '?')[0].toUpperCase()}
				</div>
			{/if}
			<span class="peer-name">
				{conversation?.peerDisplayName || conversation?.peerUsername || 'Unknown'}
			</span>
		</div>
		<span class="e2ee-badge" title="End-to-end encrypted">
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
				<rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
				<path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
			</svg>
			E2EE
		</span>
	</header>

	<!-- Message area -->
	<div class="message-area" bind:this={listEl}>
		{#if loadError}
			<div class="state-message error">Failed to load messages.</div>
		{:else if !conversation}
			<div class="state-message">Conversation not found.</div>
		{:else}
			<MessageList messages={$messages$} />
		{/if}
	</div>

	<!-- Input -->
	<MessageInput
		disabled={sending || !conversation}
		placeholder="Message {conversation?.peerDisplayName || conversation?.peerUsername || ''}…"
		on:send={handleSend}
	/>
</div>

<style>
	.dm-page {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: var(--color-bg-base);
	}

	.dm-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg-elevated);
		flex-shrink: 0;
	}

	.peer-info {
		display: flex;
		align-items: center;
		gap: 0.625rem;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		object-fit: cover;
	}

	.avatar-placeholder {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--color-brand);
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.875rem;
		font-weight: 700;
		flex-shrink: 0;
	}

	.peer-name {
		font-weight: 600;
		color: var(--color-text-primary);
		font-size: 0.9375rem;
	}

	.e2ee-badge {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.message-area {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}

	.state-message {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}

	.state-message.error {
		color: #fca5a5;
	}
</style>
