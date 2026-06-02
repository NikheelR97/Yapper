<script lang="ts">
	import { page } from "$app/stores";
	import { afterNavigate } from "$app/navigation";
	import { afterUpdate } from "svelte";
	import { get } from "svelte/store";
	import {
		conversationsStore,
		getMessageStore,
		loadMessages,
		createDmHistoryLoader,
		sendMessage,
	} from "$stores/conversations.js";
	import MessageList from "$lib/components/chat/MessageList.svelte";
	import MessageInput from "$lib/components/chat/MessageInput.svelte";
	import SafetyNumbers from "$lib/components/chat/SafetyNumbers.svelte";
	import UserAvatar from "$lib/components/UserAvatar.svelte";
	import { getPresence } from "$stores/presence.js";
	import { loadPeerTrustFlags } from "$signal/keystore.js";
	import { isNearBottom, prefersReducedMotion } from "$lib/utils/scroll.js";

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
	const dmHistoryLoader = createDmHistoryLoader(loadMessages);

	// Auto-scroll only when the reader is already at the latest message, so
	// incoming messages / presence updates don't yank them away from the
	// history they're reading. Opening a conversation always snaps to bottom.
	let atBottom = true;
	let showJump = false;
	let forceScrollNext = false;

	function handleScroll() {
		atBottom = isNearBottom(listEl);
		showJump = !atBottom;
	}

	function jumpToLatest() {
		if (!listEl) return;
		listEl.scrollTo({
			top: listEl.scrollHeight,
			behavior: prefersReducedMotion() ? "auto" : "smooth",
		});
		atBottom = true;
		showJump = false;
	}

	function retryLoad() {
		dmHistoryLoader.invalidate();
		void dmHistoryLoader.requestLoad(
			conversationId,
			conversation?.peerId ?? null,
			(nextLoading) => {
				loading = nextLoading;
			},
			(nextLoadError) => {
				loadError = nextLoadError;
			},
		);
	}

	// Force a fresh history fetch when navigating to this page (including
	// navigating back to the same conversation after visiting another), and
	// always land the reader on the newest message for the new conversation.
	afterNavigate(() => {
		dmHistoryLoader.invalidate();
		forceScrollNext = true;
		atBottom = true;
		showJump = false;
	});

	$: if (conversation?.peerId) {
		void loadPeerTrust(conversation.peerId);
	} else {
		keyChanged = false;
	}

	$: void dmHistoryLoader.requestLoad(
		conversationId,
		conversation?.peerId ?? null,
		(nextLoading) => {
			loading = nextLoading;
		},
		(nextLoadError) => {
			loadError = nextLoadError;
		},
	);

	afterUpdate(() => {
		if (!listEl) return;
		// Opening / switching a conversation (or first paint) always snaps to
		// the newest message.
		if (forceScrollNext) {
			listEl.scrollTop = listEl.scrollHeight;
			forceScrollNext = false;
			atBottom = true;
			showJump = false;
			return;
		}
		// Otherwise, follow new content only if the reader was already at the
		// bottom; if they've scrolled up, leave their position and surface the
		// "jump to latest" affordance instead.
		if (atBottom) {
			listEl.scrollTop = listEl.scrollHeight;
		} else {
			showJump = true;
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
	<div class="message-area" bind:this={listEl} on:scroll={handleScroll}>
		{#if loadError}
			<div class="state-message error" role="alert">
				<p>Couldn't load messages. Check your connection and try again.</p>
				<button type="button" class="retry-btn" on:click={retryLoad}>
					Try again
				</button>
			</div>
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

	{#if showJump && !loadError && conversation}
		<button type="button" class="jump-latest" on:click={jumpToLatest}>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2.5"
				stroke-linecap="round"
				stroke-linejoin="round"
				aria-hidden="true"
			>
				<polyline points="6 9 12 15 18 9" />
			</svg>
			Jump to latest
		</button>
	{/if}

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
		position: relative;
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
		color: var(--color-text-secondary);
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
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 1rem;
		text-align: center;
		color: var(--color-text-secondary);
		font-size: 0.875rem;
	}

	.state-message.error {
		color: var(--color-error);
	}

	.state-message.error p {
		margin: 0;
		max-width: 32ch;
	}

	.retry-btn {
		border: 1px solid var(--color-border);
		background: var(--color-bg-elevated);
		color: var(--color-text-primary);
		font: inherit;
		font-weight: 600;
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius-full);
		cursor: pointer;
		transition: background var(--transition-fast), border-color var(--transition-fast);
	}

	.retry-btn:hover {
		background: var(--color-brand);
		border-color: transparent;
		color: #fff;
	}

	.retry-btn:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.4);
	}

	/* Floating affordance shown only while the reader has scrolled up. */
	.jump-latest {
		position: absolute;
		left: 50%;
		bottom: 5.25rem;
		transform: translateX(-50%);
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.4rem 0.875rem;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-full);
		background: var(--color-bg-elevated);
		color: var(--color-text-primary);
		font: inherit;
		font-size: 0.8125rem;
		font-weight: 600;
		cursor: pointer;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
		z-index: 10;
		animation: jump-in var(--transition-base) cubic-bezier(0, 0, 0.2, 1);
		transition: background var(--transition-fast), border-color var(--transition-fast);
	}

	.jump-latest:hover {
		background: var(--color-brand);
		border-color: transparent;
		color: #fff;
	}

	.jump-latest:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.4);
	}

	@keyframes jump-in {
		from {
			opacity: 0;
			transform: translate(-50%, 0.5rem);
		}
		to {
			opacity: 1;
			transform: translate(-50%, 0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.jump-latest {
			animation: none;
		}
	}
</style>
