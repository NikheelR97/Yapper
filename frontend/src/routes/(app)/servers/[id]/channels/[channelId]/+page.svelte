<script lang="ts">
	import { page } from '$app/stores';
	import { onMount, afterUpdate } from 'svelte';
	import { getChannelMessageStore, loadChannelMessages, sendMessage, fetchChannels } from '$stores/servers.js';
	import type { Channel } from '$stores/servers.js';
	import { prepareChannel } from '$signal/index.js';
	import ServerSidebar from '$lib/components/chat/ServerSidebar.svelte';
	import MessageList from '$lib/components/chat/MessageList.svelte';
	import MessageInput from '$lib/components/chat/MessageInput.svelte';
	import TypingIndicator from '$lib/components/chat/TypingIndicator.svelte';

	$: serverId = $page.params.id ?? '';
	$: channelId = $page.params.channelId ?? '';

	$: messages$ = getChannelMessageStore(channelId);

	let sending = false;
	let loadError = false;
	let preparing = true;
	let listEl: HTMLDivElement;
	let channelName = '';

	// Re-run prepare + load whenever the channel changes
	$: if (channelId) {
		prepareAndLoad(channelId);
	}

	async function prepareAndLoad(chId: string) {
		preparing = true;
		loadError = false;
		try {
			// Resolve channel name from cache (fire-and-forget, best-effort)
			fetchChannels(serverId).then((chs: Channel[]) => {
				const ch = chs.find((c) => c.id === chId);
				if (ch) channelName = ch.name;
			}).catch(() => {});

			await prepareChannel(chId);
			await loadChannelMessages(chId);
		} catch {
			loadError = true;
		} finally {
			preparing = false;
		}
	}

	onMount(() => {
		if (channelId) prepareAndLoad(channelId);
	});

	afterUpdate(() => {
		if (listEl) listEl.scrollTop = listEl.scrollHeight;
	});

	async function handleSend(e: CustomEvent<string>) {
		sending = true;
		try {
			await sendMessage(channelId, e.detail);
		} finally {
			sending = false;
		}
	}
</script>

<svelte:head>
	<title>#{channelName || '…'} — Yapper</title>
</svelte:head>

<div class="channel-page">
	<ServerSidebar activeServerId={serverId} activeChannelId={channelId} />

	<div class="chat-area">
		<!-- Header -->
		<header class="chat-header">
			<span class="channel-name">#{channelName || '…'}</span>
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
				<div class="state-msg error">Failed to load messages.</div>
			{:else if preparing}
				<div class="state-msg">Setting up encryption…</div>
			{:else}
				<MessageList messages={$messages$} />
			{/if}
		</div>

		<!-- Typing indicator sits just above the input -->
		<TypingIndicator {channelId} />

		<!-- Input -->
		<MessageInput
			disabled={sending || preparing}
			placeholder={channelName ? `Message #${channelName}…` : 'Message…'}
			channelId={channelId}
			on:send={handleSend}
		/>
	</div>
</div>

<style>
	.channel-page {
		display: flex;
		height: 100%;
		overflow: hidden;
	}

	.chat-area {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		background: var(--color-bg-base);
	}

	.chat-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg-elevated);
		flex-shrink: 0;
	}

	.channel-name {
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

	.state-msg {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}

	.state-msg.error { color: #fca5a5; }
</style>
