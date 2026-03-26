<script lang="ts">
	import { onDestroy } from "svelte";
	import { get } from "svelte/store";
	import { onWsMessage } from "$lib/stores/ws.js";
	import { authStore } from "$stores/auth.js";
	import { readReceiptsEnabled } from "./readReceiptMode.js";

	export let messageId: string;
	export let mode: "dm" | "channel" = "dm";
	export let initialReaders: { userId: string; readAt?: string }[] = [];

	let readers = new Map<string, string>(
		initialReaders.map((reader) => [reader.userId, reader.readAt ?? ""]),
	);

	const myUserId = get(authStore).user?.id ?? "";
	const unsubscribe = onWsMessage("read_receipt", (raw) => {
		if (!readReceiptsEnabled(mode)) {
			return;
		}

		const event = raw as {
			message_id: string;
			user_id: string;
			read_at?: string;
		};
		if (event.message_id !== messageId || event.user_id === myUserId) {
			return;
		}

		readers.set(event.user_id, event.read_at ?? new Date().toISOString());
		readers = new Map(readers);
	});

	onDestroy(unsubscribe);

	$: readCount = readers.size;
</script>

{#if readReceiptsEnabled(mode) && readCount > 0}
	<span class="read-receipt">Seen by {readCount}</span>
{/if}

<style>
	.read-receipt {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		font-size: 11px;
		color: #a78bfa;
		opacity: 0.8;
		user-select: none;
	}
</style>
