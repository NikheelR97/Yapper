<script lang="ts">
    import { getPresence } from "$stores/presence.js";

    /** UUID of the user whose avatar this is. */
    export let userId: string;
    /** URL of the avatar image. Pass null/undefined for initials fallback. */
    export let avatarUrl: string | null | undefined = undefined;
    /** Display name or username — used for initials fallback and alt text. */
    export let name: string = "?";
    /** Pixel size of the avatar circle. */
    export let size: number = 32;
    /** Whether to show the presence dot. */
    export let showPresence: boolean = true;

    $: presence = getPresence(userId);
    $: initial = (name[0] ?? "?").toUpperCase();

    // Dot size scales proportionally
    $: dotSize = Math.max(8, Math.round(size * 0.3));
    $: dotOffset = Math.round(dotSize * 0.1);
</script>

<div
    class="avatar-wrap"
    style="--sz:{size}px; --dot:{dotSize}px; --dot-off:{dotOffset}px;"
    title={name}
>
    {#if avatarUrl}
        <img
            src={avatarUrl}
            alt={name}
            class="avatar-img"
            width={size}
            height={size}
        />
    {:else}
        <div class="avatar-init" aria-hidden="true">{initial}</div>
    {/if}

    {#if showPresence}
        <span
            class="presence-dot"
            class:online={$presence.online && !$presence.away}
            class:away={$presence.online && $presence.away}
            aria-label={$presence.online ? ($presence.away ? "Away" : "Online") : "Offline"}
            role="img"
        ></span>
    {/if}
</div>

<style>
    .avatar-wrap {
        position: relative;
        display: inline-flex;
        flex-shrink: 0;
        width: var(--sz);
        height: var(--sz);
    }

    .avatar-img {
        width: var(--sz);
        height: var(--sz);
        border-radius: 50%;
        object-fit: cover;
        display: block;
    }

    .avatar-init {
        width: var(--sz);
        height: var(--sz);
        border-radius: 50%;
        background: var(--color-brand, #7c3aed);
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: calc(var(--sz) * 0.4);
        font-weight: 700;
        user-select: none;
    }

    /* Presence dot — bottom-right corner */
    .presence-dot {
        position: absolute;
        bottom: calc(-1 * var(--dot-off));
        right: calc(-1 * var(--dot-off));
        width: var(--dot);
        height: var(--dot);
        border-radius: 50%;
        background: #6b7280; /* gray-500 — offline */
        border: 2px solid var(--color-bg-elevated, #1a1a2e);
        transition: background 0.3s ease;
        pointer-events: none;
    }

    .presence-dot.online {
        background: #22c55e; /* green-500 */
    }

    .presence-dot.away {
        background: #f59e0b; /* amber-400 */
    }
</style>
