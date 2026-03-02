<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import type { Message } from "$stores/conversations.js";
	import { authStore } from "$stores/auth.js";
	import { get } from "svelte/store";
	import { sendMarkRead } from "$stores/ws.js";
	import YapMessage from "./YapMessage.svelte";
	import ClipMessage from "./ClipMessage.svelte";
	import ReadReceipt from "./ReadReceipt.svelte";

	export let messages: Message[];
	/** Needed for sendMarkRead events. */
	export let channelId: string;
	/** 'dm' | 'channel' — controls ReadReceipt display mode. */
	export let mode: "dm" | "channel" = "channel";

	const myId = get(authStore).user?.id ?? "";

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleTimeString([], {
			hour: "2-digit",
			minute: "2-digit",
		});
	}

	// ── Media payload parsing ─────────────────────────────────────────────────

	function parseMediaPayload(text: string | null): {
		object_key: string;
		key: string;
		iv: string;
		mime_type: string;
	} | null {
		if (!text) return null;
		try {
			const p = JSON.parse(text);
			if (p.object_key && p.key && p.iv && p.mime_type) return p;
		} catch {
			/* not a media payload */
		}
		return null;
	}

	// ── IntersectionObserver for read receipts ───────────────────────────────

	let listEl: HTMLDivElement;
	let observer: IntersectionObserver;

	onMount(() => {
		observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (!entry.isIntersecting) continue;
					const msgId = (entry.target as HTMLElement).dataset.msgId;
					if (msgId) {
						sendMarkRead(msgId, channelId);
						observer.unobserve(entry.target); // only fire once per message
					}
				}
			},
			{ root: listEl, threshold: 0.5 },
		);
	});

	onDestroy(() => observer?.disconnect());

	/** Svelte action: attach observer when a message element is mounted. */
	function observe(node: HTMLElement, msgId: string) {
		node.dataset.msgId = msgId;
		observer?.observe(node);
		return {
			destroy() {
				observer?.unobserve(node);
			},
		};
	}
</script>

<div
	class="message-list"
	bind:this={listEl}
	role="log"
	aria-live="polite"
	aria-label="Messages"
>
	{#each messages as msg (msg.id)}
		{@const isOwn = msg.senderId === myId}
		{@const mediaPayload = parseMediaPayload(msg.text)}

		<div class="message" class:own={isOwn} use:observe={msg.id}>
			{#if msg.decryptError}
				<span class="bubble error" title="Decryption failed"
					>🔒 Unable to decrypt</span
				>
			{:else if msg.messageType === "yap" && mediaPayload}
				<YapMessage payload={mediaPayload} />
			{:else if msg.messageType === "clip" && mediaPayload}
				<ClipMessage payload={mediaPayload} />
			{:else if msg.text === null}
				<span class="bubble loading">…</span>
			{:else}
				<span class="bubble">{msg.text}</span>
			{/if}

			<div class="meta">
				<time class="timestamp" datetime={msg.createdAt}
					>{formatTime(msg.createdAt)}</time
				>
				{#if isOwn}
					<ReadReceipt messageId={msg.id} {mode} />
				{/if}
			</div>
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

	.meta {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.timestamp {
		color: var(--color-text-muted);
		font-size: 0.6875rem;
		padding: 0 0.25rem;
	}
</style>
