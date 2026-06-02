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
	import { prepareChannel, isChannelPrepared } from "$signal/index.js";
	import { isNearBottom, prefersReducedMotion } from "$lib/utils/scroll.js";

	import MessageList from "$lib/components/chat/MessageList.svelte";
	import MessageInput from "$lib/components/chat/MessageInput.svelte";
	import TypingIndicator from "$lib/components/chat/TypingIndicator.svelte";
	import LiveCanvas from "$lib/components/canvas/LiveCanvas.svelte";
	import ChannelHeader from "$lib/components/chat/ChannelHeader.svelte";
	import Skeleton from "$lib/components/Skeleton.svelte";

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

	// Auto-scroll only when the reader is already at the latest message, so
	// incoming messages / typing / presence updates don't yank them away from
	// the history they're reading. A fresh channel load always snaps to bottom.
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

	// Re-run prepare + load whenever the channel changes
	$: if (channelId) {
		prepareAndLoad(channelId);
	}

	async function prepareAndLoad(chId: string) {
		loadError = false;
		// Switching channels should always land the reader on the newest message.
		forceScrollNext = true;
		atBottom = true;
		showJump = false;
		// If channel was already prepared this session, skip the loading state
		// and show cached messages immediately while refreshing in the background.
		const alreadyPrepared = isChannelPrepared(chId);
		preparing = !alreadyPrepared;
		try {
			// Resolve channel name from cache (fire-and-forget, best-effort)
			fetchChannels(serverId)
				.then((chs: Channel[]) => {
					const ch = chs.find((c) => c.id === chId);
					if (ch) channelName = ch.name;
				})
				.catch((err) => {
					console.warn('[channel] Failed to fetch channel list:', err);
				});

			if (alreadyPrepared) {
				// Load messages first (instant), prepare in background
				await loadChannelMessages(chId);
				void prepareChannel(chId);
			} else {
				await prepareChannel(chId);
				await loadChannelMessages(chId);
			}
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
		if (!listEl) return;
		// A channel switch (or first paint) always snaps to the newest message.
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
		<div class="message-area" bind:this={listEl} on:scroll={handleScroll}>
			{#if loadError}
				<div class="state-msg error" role="alert">
					<p>Couldn't load messages. Check your connection and try again.</p>
					<button
						type="button"
						class="retry-btn"
						on:click={() => prepareAndLoad(channelId)}
					>
						Try again
					</button>
				</div>
			{:else if preparing}
				<div class="msg-skeletons" aria-hidden="true">
					{#each Array(6) as _, i}
						<div class="msg-skeleton-row">
							<Skeleton width="38px" height="38px" circle />
							<div class="msg-skeleton-body">
								<Skeleton width={i % 2 === 0 ? "30%" : "22%"} height="0.8125rem" />
								<Skeleton width={i % 3 === 0 ? "75%" : "55%"} height="0.9375rem" />
							</div>
						</div>
					{/each}
				</div>
				<p class="encrypting-note">Setting up encryption…</p>
			{:else}
				<MessageList messages={$messages$} {channelId} mode="channel" {serverEmojis} />
			{/if}
		</div>

		{#if showJump && !loadError && !preparing}
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
		<LiveCanvas {serverId} {channelId} />
	{/if}
</div>

<style>
	.channel-page {
		display: flex;
		height: 100%;
		overflow: hidden;
	}

	.chat-area {
		position: relative;
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

	.msg-skeletons {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
		padding: 1.25rem 1rem;
	}
	.msg-skeleton-row {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
	}
	.msg-skeleton-body {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		flex: 1;
		min-width: 0;
	}
	.encrypting-note {
		margin: 0 auto 1rem;
		font-size: 0.75rem;
		color: var(--color-text-secondary);
	}

	.state-msg {
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

	.state-msg.error {
		color: var(--color-error);
	}

	.state-msg.error p {
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
