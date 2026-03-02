<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { onWsMessage } from "$lib/stores/ws.js";
    import { authStore } from "$stores/auth.js";
    import { get } from "svelte/store";

    /** The message whose receipt state we're tracking. */
    export let messageId: string;
    /** 'dm' shows "Read HH:MM", 'channel' shows "{n} read". */
    export let mode: "dm" | "channel" = "dm";
    /**
     * Initial read-by user IDs (loaded from history).
     * Exclude the sender's own ID before passing in.
     */
    export let initialReaders: { userId: string; readAt?: string }[] = [];

    // ── State ──────────────────────────────────────────────────────────────────
    // Map of userId → ISO timestamp (or empty string if unknown)
    let readers = new Map<string, string>(
        initialReaders.map((r) => [r.userId, r.readAt ?? ""]),
    );

    // ── Live updates ──────────────────────────────────────────────────────────
    const myUserId = get(authStore).user?.id ?? "";

    const unsubscribe = onWsMessage("read_receipt", (raw) => {
        const evt = raw as {
            message_id: string;
            user_id: string;
            read_at?: string;
        };
        if (evt.message_id !== messageId) return;
        if (evt.user_id === myUserId) return; // skip own receipt
        readers.set(evt.user_id, evt.read_at ?? new Date().toISOString());
        readers = new Map(readers); // trigger reactivity
    });

    onDestroy(unsubscribe);

    // ── Derived display values ────────────────────────────────────────────────
    $: readCount = readers.size;

    $: lastReadAt = (() => {
        if (readers.size === 0) return null;
        const times = [...readers.values()].filter(Boolean).sort();
        const iso = times.at(-1);
        if (!iso) return null;
        const d = new Date(iso);
        return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    })();
</script>

<!-- ReadReceipt: DMs show 'Read HH:MM', channels show 'N read'. Subscribes to WS read_receipt events. -->

{#if readCount > 0}
    <span class="read-receipt">
        {#if mode === "dm"}
            ✓✓ Read {lastReadAt ?? ""}
        {:else}
            ✓✓ {readCount} read
        {/if}
    </span>
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
