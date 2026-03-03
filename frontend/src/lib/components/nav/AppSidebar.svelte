<script lang="ts">
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import {
        serversStore,
        fetchServers,
        fetchChannels,
        createServer,
        createInvite,
        joinByInvite,
    } from "$stores/servers.js";
    import type { Channel } from "$stores/servers.js";
    import { conversationsStore } from "$stores/conversations.js";
    import UserAvatar from "$lib/components/UserAvatar.svelte";
    import { onMount } from "svelte";

    $: path = $page.url.pathname;
    $: serverId = $page.params.id ?? "";
    $: channelId = $page.params.channelId ?? "";

    // Sidebar mode based on route
    $: mode =
        path.startsWith("/servers") && serverId
            ? "server"
            : path.startsWith("/dm")
              ? "dm"
              : "default";

    // ── Server mode state ──
    let channels: Channel[] = [];
    let loadingChannels = false;

    // ── Create server modal ──
    let showCreateModal = false;
    let newServerName = "";
    let creating = false;

    // ── Invite panel ──
    let inviteCode = "";
    let showInvitePanel = false;

    // ── Join by invite ──
    let joinCode = "";
    let joining = false;
    let joinError = "";

    onMount(async () => {
        if (!$serversStore.servers.length) {
            await fetchServers();
        }
    });

    $: if (serverId && mode === "server") {
        loadChannels(serverId);
    }

    async function loadChannels(sid: string) {
        loadingChannels = true;
        try {
            channels = await fetchChannels(sid);
        } catch {
            channels = [];
        } finally {
            loadingChannels = false;
        }
    }

    function serverInitial(name: string): string {
        return name.slice(0, 2).toUpperCase();
    }

    function navigateToChannel(sid: string, cid: string) {
        goto(`/servers/${sid}/channels/${cid}`);
    }

    async function handleCreateServer() {
        if (!newServerName.trim() || creating) return;
        creating = true;
        try {
            const server = await createServer(newServerName.trim());
            newServerName = "";
            showCreateModal = false;
            goto(`/servers/${server.id}/channels`);
        } finally {
            creating = false;
        }
    }

    async function handleGetInvite() {
        if (!serverId) return;
        inviteCode = await createInvite(serverId);
        showInvitePanel = true;
    }

    function copyInvite() {
        navigator.clipboard.writeText(inviteCode).catch(() => {});
    }

    async function handleJoinByInvite() {
        if (!joinCode.trim() || joining) return;
        joining = true;
        joinError = "";
        try {
            await joinByInvite(joinCode.trim());
            joinCode = "";
        } catch {
            joinError = "Invalid or expired invite code.";
        } finally {
            joining = false;
        }
    }
</script>

<aside class="sidebar" aria-label="App sidebar">
    {#if mode === "server"}
        <!-- ═══ SERVER MODE ═══ -->
        <!-- Server icon strip -->
        <nav class="server-strip" aria-label="Servers">
            <!-- Home button -->
            <button
                class="server-btn home-btn"
                class:active={!serverId}
                on:click={() => goto("/explore")}
                title="Home"
                aria-label="Home"
            >
                <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                >
                    <path
                        d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
                    /><polyline points="9 22 9 12 15 12 15 22" />
                </svg>
            </button>

            <div class="strip-divider" aria-hidden="true"></div>

            {#each $serversStore.servers as server (server.id)}
                <button
                    class="server-btn"
                    class:active={server.id === serverId}
                    on:click={() => {
                        if (server.id !== serverId) {
                            goto(`/servers/${server.id}/channels`);
                        }
                    }}
                    title={server.name}
                    aria-label={server.name}
                    aria-current={server.id === serverId ? "page" : undefined}
                >
                    {#if server.iconUrl}
                        <img
                            src={server.iconUrl}
                            alt={server.name}
                            width="36"
                            height="36"
                        />
                    {:else}
                        <span aria-hidden="true"
                            >{serverInitial(server.name)}</span
                        >
                    {/if}
                </button>
            {/each}

            <button
                class="server-btn add-btn"
                on:click={() => (showCreateModal = true)}
                title="Create or join a server"
                aria-label="Create or join a server"
            >
                <svg
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                >
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
            </button>
        </nav>

        <!-- Channel list panel -->
        <div class="channel-panel">
            <header class="channel-panel-header">
                <span class="panel-title"
                    >{$serversStore.servers.find((s) => s.id === serverId)
                        ?.name ?? "…"}</span
                >
                {#if $serversStore.servers.find((s) => s.id === serverId)}
                    <button
                        class="icon-sm"
                        on:click={handleGetInvite}
                        title="Create invite link"
                        aria-label="Create invite link"
                    >
                        <svg
                            width="13"
                            height="13"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            aria-hidden="true"
                        >
                            <path
                                d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"
                            ></path>
                            <path
                                d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"
                            ></path>
                        </svg>
                    </button>
                {/if}
            </header>

            {#if showInvitePanel}
                <div class="invite-panel">
                    <span class="invite-code">{inviteCode}</span>
                    <button class="pill-btn" on:click={copyInvite}>Copy</button>
                    <button
                        class="pill-btn"
                        on:click={() => (showInvitePanel = false)}
                        aria-label="Close">✕</button
                    >
                </div>
            {/if}

            {#if loadingChannels}
                <div class="panel-empty">Loading…</div>
            {:else if channels.length === 0}
                <div class="panel-empty">No channels yet.</div>
            {:else}
                <ul class="channel-list" role="list">
                    {#each channels as ch (ch.id)}
                        <li>
                            <button
                                class="channel-btn"
                                class:active={ch.id === channelId}
                                on:click={() =>
                                    navigateToChannel(serverId, ch.id)}
                                aria-current={ch.id === channelId
                                    ? "page"
                                    : undefined}
                            >
                                <span class="channel-hash" aria-hidden="true"
                                    >#</span
                                >
                                {ch.name}
                            </button>
                        </li>
                    {/each}
                </ul>
            {/if}

            <!-- Join by invite -->
            <div class="join-section">
                <form
                    on:submit|preventDefault={handleJoinByInvite}
                    class="join-form"
                >
                    <input
                        bind:value={joinCode}
                        placeholder="Invite code…"
                        class="join-input"
                        disabled={joining}
                        aria-label="Enter invite code"
                    />
                    <button
                        type="submit"
                        class="join-submit"
                        disabled={joining || !joinCode.trim()}>Join</button
                    >
                </form>
                {#if joinError}
                    <p class="join-error">{joinError}</p>
                {/if}
            </div>
        </div>
    {:else if mode === "dm"}
        <!-- ═══ DM MODE ═══ -->
        <div class="dm-sidebar">
            <header class="dm-sidebar-header">
                <span class="panel-title">Direct Messages</span>
                <!-- Placeholder for "New DM" button -->
            </header>

            {#if $conversationsStore.loading}
                <div class="panel-empty">Loading…</div>
            {:else if $conversationsStore.conversations.length === 0}
                <div class="panel-empty empty-dm">
                    <p>No conversations yet.</p>
                    <p class="hint">
                        Search for people on the Explore page to start a DM.
                    </p>
                </div>
            {:else}
                <ul class="conv-list" role="list">
                    {#each $conversationsStore.conversations as conv (conv.id)}
                        <li>
                            <button
                                class="conv-btn"
                                class:active={$page.params.conversationId ===
                                    conv.id}
                                on:click={() => goto(`/dm/${conv.id}`)}
                                aria-current={$page.params.conversationId ===
                                conv.id
                                    ? "page"
                                    : undefined}
                            >
                                <div class="conv-avatar">
                                    <UserAvatar
                                        userId={conv.peerId}
                                        avatarUrl={conv.peerAvatarUrl}
                                        name={conv.peerDisplayName ||
                                            conv.peerUsername ||
                                            "?"}
                                        size={34}
                                    />
                                </div>
                                <div class="conv-info">
                                    <span class="conv-name"
                                        >{conv.peerDisplayName ||
                                            conv.peerUsername}</span
                                    >
                                    {#if conv.lastMessageAt}
                                        <span class="conv-time"
                                            >{new Date(
                                                conv.lastMessageAt,
                                            ).toLocaleDateString(undefined, {
                                                month: "short",
                                                day: "numeric",
                                            })}</span
                                        >
                                    {/if}
                                </div>
                            </button>
                        </li>
                    {/each}
                </ul>
            {/if}
        </div>
    {:else}
        <!-- ═══ DEFAULT MODE (explore, settings, profile) ═══ -->
        <div class="default-sidebar">
            <header class="default-sidebar-header">
                <span class="panel-title">Navigation</span>
            </header>

            <nav class="quick-nav" aria-label="Quick navigation">
                <a
                    class="quick-link"
                    class:active={path.startsWith("/explore")}
                    href="/explore"
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                    >
                        <circle cx="12" cy="12" r="10" /><polygon
                            points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"
                        />
                    </svg>
                    Explore
                </a>
                <a
                    class="quick-link"
                    class:active={path.startsWith("/dm")}
                    href="/dm"
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                    >
                        <path
                            d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
                        />
                    </svg>
                    Direct Messages
                </a>
                <a
                    class="quick-link"
                    class:active={path.startsWith("/settings")}
                    href="/settings"
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                    >
                        <circle cx="12" cy="12" r="3" /><path
                            d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09"
                        />
                    </svg>
                    Settings
                </a>
            </nav>

            {#if $serversStore.servers.length > 0}
                <div class="servers-section">
                    <span class="section-label">Your Servers</span>
                    <ul class="server-list" role="list">
                        {#each $serversStore.servers as server (server.id)}
                            <li>
                                <a
                                    class="server-link"
                                    href="/servers/{server.id}/channels"
                                >
                                    <span class="server-icon-sm">
                                        {#if server.iconUrl}
                                            <img
                                                src={server.iconUrl}
                                                alt=""
                                                width="24"
                                                height="24"
                                            />
                                        {:else}
                                            {serverInitial(server.name)}
                                        {/if}
                                    </span>
                                    <span class="server-name-text"
                                        >{server.name}</span
                                    >
                                </a>
                            </li>
                        {/each}
                    </ul>
                </div>
            {/if}

            <button
                class="create-server-btn"
                on:click={() => (showCreateModal = true)}
            >
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
                    <line x1="12" y1="5" x2="12" y2="19"></line>
                    <line x1="5" y1="12" x2="19" y2="12"></line>
                </svg>
                Create Server
            </button>
        </div>
    {/if}
</aside>

<!-- ═══ Create Server Modal ═══ -->
{#if showCreateModal}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click={() => (showCreateModal = false)}>
        <div
            class="modal"
            role="dialog"
            aria-modal="true"
            aria-label="Create server"
            on:click|stopPropagation
        >
            <h2 class="modal-title">Create a Server</h2>
            <form on:submit|preventDefault={handleCreateServer}>
                <input
                    bind:value={newServerName}
                    placeholder="Server name"
                    class="modal-input"
                    maxlength="100"
                    required
                    autofocus
                />
                <div class="modal-actions">
                    <button
                        type="button"
                        class="modal-cancel"
                        on:click={() => (showCreateModal = false)}
                        >Cancel</button
                    >
                    <button
                        type="submit"
                        class="modal-submit"
                        disabled={creating || !newServerName.trim()}
                    >
                        {creating ? "Creating…" : "Create"}
                    </button>
                </div>
            </form>
        </div>
    </div>
{/if}

<style>
    .sidebar {
        display: flex;
        flex-shrink: 0;
        height: 100%;
        background: var(--color-bg-base);
        border-right: 1px solid var(--color-border);
        width: 240px;
        overflow: hidden;
    }

    /* ═══ shared ═══ */
    .panel-title {
        font-size: 0.8125rem;
        font-weight: 700;
        color: var(--color-text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .panel-empty {
        padding: 1rem 0.75rem;
        font-size: 0.8125rem;
        color: var(--color-text-muted);
    }

    .icon-sm {
        background: none;
        border: none;
        color: var(--color-text-muted);
        cursor: pointer;
        padding: 0.2rem;
        border-radius: 0.25rem;
        display: flex;
        align-items: center;
        flex-shrink: 0;
    }
    .icon-sm:hover {
        color: var(--color-text-primary);
    }

    .pill-btn {
        background: none;
        border: 1px solid var(--color-border);
        border-radius: 0.25rem;
        padding: 0.125rem 0.375rem;
        font-size: 0.625rem;
        cursor: pointer;
        color: var(--color-text-muted);
        flex-shrink: 0;
    }
    .pill-btn:hover {
        color: var(--color-text-primary);
    }

    /* ═══ SERVER MODE ═══ */
    .server-strip {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.375rem;
        padding: 0.5rem 0.375rem;
        width: 52px;
        background: #0d0d14;
        border-right: 1px solid var(--color-border);
        overflow-y: auto;
        scrollbar-width: none;
        flex-shrink: 0;
    }
    .server-strip::-webkit-scrollbar {
        display: none;
    }

    .strip-divider {
        width: 24px;
        height: 1px;
        background: var(--color-border);
        margin: 0.125rem 0;
        flex-shrink: 0;
    }

    .server-btn {
        width: 36px;
        height: 36px;
        border-radius: 50%;
        border: none;
        background: var(--color-bg-surface);
        color: var(--color-text-primary);
        cursor: pointer;
        font-size: 0.625rem;
        font-weight: 700;
        display: flex;
        align-items: center;
        justify-content: center;
        transition:
            border-radius 0.15s,
            background 0.15s;
        overflow: hidden;
        flex-shrink: 0;
    }
    .server-btn:hover,
    .server-btn.active {
        border-radius: 0.625rem;
        background: var(--color-brand);
        color: white;
    }
    .server-btn img {
        width: 36px;
        height: 36px;
        object-fit: cover;
    }
    .home-btn {
        background: var(--color-bg-elevated);
        color: var(--color-text-secondary);
    }
    .add-btn {
        background: var(--color-bg-elevated);
        color: var(--color-brand);
    }
    .add-btn:hover {
        background: var(--color-brand);
        color: white;
    }

    .channel-panel {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-width: 0;
        overflow: hidden;
    }

    .channel-panel-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem 0.625rem 0.5rem;
        border-bottom: 1px solid var(--color-border);
        flex-shrink: 0;
    }

    .invite-panel {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.375rem 0.625rem;
        background: var(--color-bg-surface);
        border-bottom: 1px solid var(--color-border);
        font-size: 0.6875rem;
        flex-shrink: 0;
    }
    .invite-code {
        flex: 1;
        font-family: monospace;
        color: var(--color-text-primary);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 0.625rem;
    }

    .channel-list {
        list-style: none;
        padding: 0.25rem 0;
        overflow-y: auto;
        flex: 1;
    }

    .channel-btn {
        width: calc(100% - 0.625rem);
        background: none;
        border: none;
        display: flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem 0.5rem;
        border-radius: 0.25rem;
        margin: 0 0.3125rem;
        font-size: 0.8125rem;
        color: var(--color-text-muted);
        cursor: pointer;
        text-align: left;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        transition:
            background 0.1s,
            color 0.1s;
    }
    .channel-btn:hover {
        background: var(--color-bg-surface);
        color: var(--color-text-primary);
    }
    .channel-btn.active {
        background: var(--color-bg-elevated);
        color: var(--color-text-primary);
        font-weight: 600;
    }
    .channel-hash {
        color: var(--color-text-muted);
        font-size: 0.875rem;
        flex-shrink: 0;
    }

    .join-section {
        padding: 0.5rem 0.625rem;
        border-top: 1px solid var(--color-border);
        flex-shrink: 0;
    }
    .join-form {
        display: flex;
        gap: 0.25rem;
    }
    .join-input {
        flex: 1;
        background: var(--color-bg-surface);
        border: 1px solid var(--color-border);
        border-radius: 0.375rem;
        padding: 0.25rem 0.5rem;
        font-size: 0.6875rem;
        color: var(--color-text-primary);
        min-width: 0;
    }
    .join-input::placeholder {
        color: var(--color-text-muted);
    }
    .join-submit {
        background: var(--color-brand);
        border: none;
        border-radius: 0.375rem;
        padding: 0.25rem 0.5rem;
        font-size: 0.6875rem;
        color: white;
        cursor: pointer;
        font-weight: 600;
        flex-shrink: 0;
    }
    .join-submit:disabled {
        opacity: 0.5;
        cursor: default;
    }
    .join-error {
        margin: 0.25rem 0 0;
        font-size: 0.625rem;
        color: #fca5a5;
    }

    /* ═══ DM MODE ═══ */
    .dm-sidebar {
        width: 100%;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .dm-sidebar-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem 0.75rem 0.5rem;
        border-bottom: 1px solid var(--color-border);
        flex-shrink: 0;
    }

    .empty-dm {
        text-align: center;
    }
    .hint {
        font-size: 0.75rem;
        color: var(--color-text-muted);
        margin-top: 0.25rem;
    }

    .conv-list {
        list-style: none;
        overflow-y: auto;
        flex: 1;
        padding: 0.25rem 0;
    }

    .conv-btn {
        width: calc(100% - 0.625rem);
        background: none;
        border: none;
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.625rem;
        border-radius: 0.375rem;
        margin: 0.0625rem 0.3125rem;
        cursor: pointer;
        text-align: left;
        transition: background 0.1s;
    }
    .conv-btn:hover {
        background: rgba(255, 255, 255, 0.04);
    }
    .conv-btn.active {
        background: rgba(124, 58, 237, 0.12);
        border-left: 2px solid var(--color-brand);
    }

    .conv-avatar {
        flex-shrink: 0;
    }
    .conv-info {
        display: flex;
        flex-direction: column;
        min-width: 0;
        flex: 1;
    }
    .conv-name {
        font-size: 0.8125rem;
        font-weight: 600;
        color: var(--color-text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .conv-time {
        font-size: 0.6875rem;
        color: var(--color-text-muted);
    }

    /* ═══ DEFAULT MODE ═══ */
    .default-sidebar {
        width: 100%;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .default-sidebar-header {
        padding: 0.75rem 0.75rem 0.5rem;
        border-bottom: 1px solid var(--color-border);
        flex-shrink: 0;
    }

    .quick-nav {
        display: flex;
        flex-direction: column;
        padding: 0.375rem 0;
    }

    .quick-link {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.75rem;
        font-size: 0.8125rem;
        color: var(--color-text-secondary);
        text-decoration: none;
        border-radius: 0.25rem;
        margin: 0 0.375rem;
        transition:
            background 0.1s,
            color 0.1s;
    }
    .quick-link:hover {
        background: rgba(255, 255, 255, 0.04);
        color: var(--color-text-primary);
        text-decoration: none;
    }
    .quick-link.active {
        background: rgba(124, 58, 237, 0.12);
        color: var(--color-text-primary);
        font-weight: 600;
    }

    .servers-section {
        padding: 0.75rem 0.75rem 0;
        border-top: 1px solid var(--color-border);
        flex: 1;
        overflow-y: auto;
    }

    .section-label {
        font-size: 0.625rem;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.07em;
        color: var(--color-text-muted);
        display: block;
        margin-bottom: 0.375rem;
    }

    .server-list {
        list-style: none;
    }

    .server-link {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.375rem 0;
        font-size: 0.8125rem;
        color: var(--color-text-secondary);
        text-decoration: none;
        transition: color 0.1s;
    }
    .server-link:hover {
        color: var(--color-text-primary);
        text-decoration: none;
    }

    .server-icon-sm {
        width: 24px;
        height: 24px;
        border-radius: 6px;
        background: var(--color-bg-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.5625rem;
        font-weight: 700;
        color: var(--color-text-primary);
        overflow: hidden;
        flex-shrink: 0;
    }
    .server-icon-sm img {
        width: 24px;
        height: 24px;
        object-fit: cover;
    }
    .server-name-text {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .create-server-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.375rem;
        margin: 0.5rem 0.75rem;
        padding: 0.5rem;
        background: rgba(124, 58, 237, 0.1);
        border: 1px dashed rgba(124, 58, 237, 0.3);
        border-radius: var(--radius-md);
        color: var(--color-brand-light);
        font-size: 0.75rem;
        font-weight: 600;
        cursor: pointer;
        transition:
            background 0.1s,
            border-color 0.1s;
        flex-shrink: 0;
    }
    .create-server-btn:hover {
        background: rgba(124, 58, 237, 0.18);
        border-color: rgba(124, 58, 237, 0.5);
    }

    /* ═══ MODAL ═══ */
    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 100;
    }

    .modal {
        background: var(--color-bg-elevated);
        border: 1px solid var(--color-border);
        border-radius: 0.75rem;
        padding: 1.5rem;
        width: 320px;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .modal-title {
        font-size: 1.125rem;
        font-weight: 700;
        color: var(--color-text-primary);
        margin: 0;
    }

    .modal-input {
        width: 100%;
        background: var(--color-bg-surface);
        border: 1px solid var(--color-border);
        border-radius: 0.5rem;
        padding: 0.625rem 0.75rem;
        font-size: 0.9375rem;
        color: var(--color-text-primary);
        box-sizing: border-box;
    }
    .modal-input::placeholder {
        color: var(--color-text-muted);
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
    }

    .modal-cancel {
        background: none;
        border: 1px solid var(--color-border);
        border-radius: 0.375rem;
        padding: 0.5rem 1rem;
        font-size: 0.875rem;
        color: var(--color-text-muted);
        cursor: pointer;
    }
    .modal-cancel:hover {
        color: var(--color-text-primary);
    }

    .modal-submit {
        background: var(--color-brand);
        border: none;
        border-radius: 0.375rem;
        padding: 0.5rem 1rem;
        font-size: 0.875rem;
        color: white;
        font-weight: 600;
        cursor: pointer;
    }
    .modal-submit:disabled {
        opacity: 0.5;
        cursor: default;
    }
</style>
