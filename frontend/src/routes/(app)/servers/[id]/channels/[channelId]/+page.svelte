<script lang="ts">
	import { page } from "$app/stores";
	import { onMount, afterUpdate } from "svelte";
	import {
		getChannelMessageStore,
		loadChannelMessages,
		sendMessage,
		fetchChannels,
		fetchServerEmojis,
		serversStore,
	} from "$stores/servers.js";
	import type { Channel } from "$stores/servers.js";
	import { prepareChannel } from "$signal/index.js";

	import MessageList from "$lib/components/chat/MessageList.svelte";
	import MessageInput from "$lib/components/chat/MessageInput.svelte";
	import TypingIndicator from "$lib/components/chat/TypingIndicator.svelte";
	import LiveCanvas from "$lib/components/canvas/LiveCanvas.svelte";
	import ChannelHeader from "$lib/components/chat/ChannelHeader.svelte";

	let showCanvas = true;

	$: serverId = $page.params.id ?? "";
	$: channelId = $page.params.channelId ?? "";

	// Load server emojis when entering a server (cached in store after first load)
	$: if (serverId) fetchServerEmojis(serverId);
	$: serverEmojis = $serversStore.servers.find((s) => s.id === serverId)?.customEmojis ?? [];

	$: messages$ = getChannelMessageStore(channelId);

	let sending = false;
	let loadError = false;
	let preparing = true;
	let listEl: HTMLDivElement;
	let channelName = "";

	// Re-run prepare + load whenever the channel changes
	$: if (channelId) {
		prepareAndLoad(channelId);
	}

	async function prepareAndLoad(chId: string) {
		preparing = true;
		loadError = false;
		try {
			// Resolve channel name from cache (fire-and-forget, best-effort)
			fetchChannels(serverId)
				.then((chs: Channel[]) => {
					const ch = chs.find((c) => c.id === chId);
					if (ch) channelName = ch.name;
				})
				.catch(() => {});

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
	<title>#{channelName || "…"} — Yapper</title>
</svelte:head>

<div class="channel-page">
	<div class="chat-area" class:canvas-open={showCanvas}>
		<!-- Header -->
		<ChannelHeader
			{channelName}
			{showCanvas}
			on:toggleCanvas={() => (showCanvas = !showCanvas)}
		/>

		<!-- Message area -->
		<div class="message-area" bind:this={listEl}>
			{#if loadError}
				<div class="state-msg error">Failed to load messages.</div>
			{:else if preparing}
				<div class="state-msg">Setting up encryption…</div>
			{:else}
				<MessageList messages={$messages$} {channelId} mode="channel" {serverEmojis} />
			{/if}
		</div>

		<!-- Typing indicator sits just above the input -->
		<TypingIndicator {channelId} />

		<!-- Input -->
		<MessageInput
			disabled={sending || preparing}
			placeholder={channelName ? `Message #${channelName}…` : "Message…"}
			{channelId}
			{serverId}
			on:send={handleSend}
		/>
	</div>

	{#if showCanvas && serverId}
		<LiveCanvas {serverId} />
	{/if}
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

	.state-msg.error {
		color: #fca5a5;
	}
</style>
