<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { serversStore, fetchServers } from "$stores/servers.js";
    import { get } from "svelte/store";

    onMount(async () => {
        if (!get(serversStore).servers.length) {
            await fetchServers();
        }
        const servers = get(serversStore).servers;
        if (servers.length > 0) {
            // Auto-navigate to the first server
            goto(`/servers/${servers[0].id}/channels`, { replaceState: true });
        }
    });
</script>

<svelte:head>
    <title>Servers — Yapper</title>
</svelte:head>

<div class="servers-index">
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
            <circle cx="12" cy="12" r="10" /><line
                x1="12"
                y1="8"
                x2="12"
                y2="16"
            /><line x1="8" y1="12" x2="16" y2="12" />
        </svg>
        <h2>No Servers Yet</h2>
        <p>Create a server or join one using an invite code to get started.</p>
    </div>
</div>

<style>
    .servers-index {
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
