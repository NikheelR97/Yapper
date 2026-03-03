<script lang="ts">
    import { onMount } from "svelte";
    import { fetchConversations } from "$stores/conversations.js";

    onMount(() => {
        fetchConversations();
    });
</script>

<svelte:head>
    <title>Direct Messages — Yapper</title>
</svelte:head>

<div class="dm-index">
    <div class="empty-state">
        <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path
                d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
            />
        </svg>
        <h2>Your Messages</h2>
        <p>
            Select a conversation from the sidebar or search for someone on the
            Explore page to start chatting.
        </p>
    </div>
</div>

<style>
    .dm-index {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        background: var(--color-bg-base);
    }

    .empty-state {
        text-align: center;
        max-width: 320px;
        color: var(--color-text-muted);
    }

    .empty-state svg {
        margin-bottom: 1rem;
        opacity: 0.4;
    }

    .empty-state h2 {
        font-size: 1.125rem;
        font-weight: 700;
        color: var(--color-text-primary);
        margin-bottom: 0.5rem;
    }

    .empty-state p {
        font-size: 0.8125rem;
        line-height: 1.5;
    }
</style>
