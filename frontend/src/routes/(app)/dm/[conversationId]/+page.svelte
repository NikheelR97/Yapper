<script lang="ts">
	import { page } from "$app/stores";
	import { onMount, afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import {
		conversationsStore,
		getMessageStore,
		loadMessages,
		sendMessage,
	} from "$stores/conversations.js";
	import MessageList from "$lib/components/chat/MessageList.svelte";
	import MessageInput from "$lib/components/chat/MessageInput.svelte";
	import SafetyNumbers from "$lib/components/chat/SafetyNumbers.svelte";
	import UserAvatar from "$lib/components/UserAvatar.svelte";
	import { getPresence } from "$stores/presence.js";
	import { loadPeerTrustFlags } from "$signal/keystore.js";

	$: conversationId = $page.params.conversationId ?? "";

	// Look up conversation metadata from the store
	$: conversation = $conversationsStore.conversations.find(
		(c) => c.id === conversationId,
	);
	$: messages$ = getMessageStore(conversationId);
	// Always a valid store; presence will stay offline if peerId is empty
	$: peerPresence = getPresence(conversation?.peerId ?? "");

	let sending = false;
	let loading = true;
	let loadError = false;
	let listEl: HTMLDivElement;
	let showSafetyNumbers = false;
	let keyChanged = false;

	$: if (conversation?.peerId) {
		void loadPeerTrust(conversation.peerId);
	} else {
		keyChanged = false;
	}

	onMount(async () => {
		if (!conversation) { loading = false; return; }
		try {
			await loadMessages(conversationId, conversation.peerId);
		} catch {
			loadError = true;
		} finally {
			loading = false;
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
		} catch (err) {
			console.error('[handleSend] sendMessage failed:', err);
			throw err;
		} finally {
			sending = false;
		}
	}

	async function loadPeerTrust(peerId: string) {
		const trust = await loadPeerTrustFlags(peerId);
		keyChanged = trust.keyChanged;
	}
</script>

<svelte:head>
	<title
		>{conversation
			? conversation.peerDisplayName || conversation.peerUsername
			: "Direct Message"} — Yapper</title
	>
</svelte:head>

<div class="dm-page">
	<!-- Header -->
	<header class="dm-header">
		<div class="peer-info">
			{#if conversation}
				<UserAvatar
					userId={conversation.peerId}
					avatarUrl={conversation.peerAvatarUrl}
					name={conversation.peerDisplayName ||
						conversation.peerUsername ||
						"?"}
					size={32}
				/>
			{:else}
				<div class="avatar-placeholder" aria-hidden="true">?</div>
			{/if}
			<div class="peer-meta">
				<span class="peer-name">
					{conversation?.peerDisplayName ||
						conversation?.peerUsername ||
						"Unknown"}
				</span>
				{#if conversation?.peerId}
					<span
						class="peer-status"
						class:online={$peerPresence.online}
					>
						{$peerPresence.online ? "Online" : "Offline"}
					</span>
				{/if}
			</div>
		</div>
		<button
			class="e2ee-badge"
			class:key-changed={keyChanged}
			title={keyChanged ? "Security code changed — click to verify" : "End-to-end encrypted — click to verify"}
			on:click={() => (showSafetyNumbers = true)}
		>
			<svg
				width="12"
				height="12"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.5"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
				<path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
			</svg>
			{keyChanged ? '⚠ Verify' : 'E2EE'}
		</button>
	</header>

	<!-- Safety Numbers modal -->
	{#if conversation}
		<SafetyNumbers
			peerId={conversation.peerId}
			peerName={conversation.peerDisplayName || conversation.peerUsername || 'them'}
			bind:open={showSafetyNumbers}
		/>
	{/if}

	<!-- Message area -->
	<div class="message-area" bind:this={listEl}>
		{#if loadError}
			<div class="state-message error">Failed to load messages.</div>
		{:else if !conversation}
			<div class="state-message">Conversation not found.</div>
		{:else}
			<MessageList
				messages={$messages$}
				channelId={conversationId}
				mode="dm"
			/>
		{/if}
	</div>

	<!-- Input -->
	<MessageInput
		disabled={sending || loading || !conversation}
		conversationId={conversation?.id}
		recipientId={conversation?.peerId}
		placeholder="Message {conversation?.peerDisplayName ||
			conversation?.peerUsername ||
			''}…"
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

	.peer-meta {
		display: flex;
		flex-direction: column;
		gap: 0.0625rem;
	}

	.peer-name {
		font-weight: 600;
		color: var(--color-text-primary);
		font-size: 0.9375rem;
		line-height: 1.2;
	}

	.peer-status {
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		line-height: 1.2;
	}
	.peer-status.online {
		color: #22c55e;
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
		background: none;
		border: 1px solid transparent;
		border-radius: 6px;
		padding: 0.25rem 0.5rem;
		cursor: pointer;
		transition: border-color 0.15s, color 0.15s;
	}
	.e2ee-badge:hover {
		border-color: var(--color-border);
		color: var(--color-text-secondary);
	}
	.e2ee-badge.key-changed {
		color: #fbbf24;
		border-color: rgba(234, 179, 8, 0.4);
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
