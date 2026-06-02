<script lang="ts">
    import { page } from "$app/stores";
    import { goto } from "$app/navigation";
    import { authStore } from "$stores/auth.js";
    import UserAvatar from "$lib/components/UserAvatar.svelte";

    $: path = $page.url.pathname;
    $: user = $authStore.user;

    $: isParent = user?.accountType === 'parent';

    const baseNavItems = [
        { label: "Channels", href: "/servers", match: "/servers" },
        { label: "Direct", href: "/dm", match: "/dm" },
        { label: "Explore", href: "/explore", match: "/explore" },
    ];

    $: navItems = isParent
        ? [...baseNavItems, { label: "Family", href: "/parent/dashboard", match: "/parent" }]
        : baseNavItems;

    function isActive(match: string): boolean {
        return path.startsWith(match);
    }

    function handleAvatarClick() {
        if (user) goto(`/profile/${user.username}`);
    }
</script>

<nav class="topnav" aria-label="Main navigation">
    <!-- Logo -->
    <a class="logo" href="/explore" aria-label="Yapper home">
        <span class="logo-sphere" aria-hidden="true"></span>
        <span class="logo-text">Yapper</span>
    </a>

    <!-- Nav links -->
    <div class="nav-links">
        {#each navItems as item}
            <a
                class="nav-link"
                class:active={isActive(item.match)}
                href={item.href}
                aria-current={isActive(item.match) ? "page" : undefined}
            >
                {item.label}
            </a>
        {/each}
    </div>

    <!-- Right actions -->
    <div class="nav-actions">
        <button
            class="icon-btn"
            title="Settings"
            aria-label="Settings"
            on:click={() => goto("/settings")}
        >
            <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
            >
                <circle cx="12" cy="12" r="3" /><path
                    d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z"
                />
            </svg>
        </button>

        <button
            type="button"
            class="user-avatar-btn"
            on:click={handleAvatarClick}
            title={user?.username ?? "Profile"}
            aria-label={user ? `Open ${user.username}'s profile` : "Open profile"}
        >
            {#if user}
                <UserAvatar
                    userId={user.id}
                    avatarUrl={user.avatarUrl ?? null}
                    name={user.displayName || user.username || "?"}
                    size={30}
                />
            {:else}
                <div class="avatar-fallback">?</div>
            {/if}
        </button>
    </div>
</nav>

<style>
    .topnav {
        display: flex;
        align-items: center;
        height: 48px;
        padding: 0 0.75rem;
        background: var(--color-bg-nav, #0d0d14);
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        flex-shrink: 0;
        gap: 0.5rem;
        z-index: 50;
    }

    /* ── Logo ── */
    .logo {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        text-decoration: none;
        color: var(--color-text-primary);
        flex-shrink: 0;
        margin-right: 0.5rem;
    }
    .logo:hover {
        text-decoration: none;
    }

    .logo-sphere {
        width: 22px;
        height: 22px;
        border-radius: 50%;
        flex-shrink: 0;
        background: radial-gradient(
            circle at 35% 35%,
            #c4b5fd 0%,
            #7c3aed 45%,
            #2e1065 100%
        );
        box-shadow:
            0 0 10px rgba(124, 58, 237, 0.45),
            inset 0 -3px 6px rgba(46, 16, 101, 0.55);
    }
    .logo-text {
        font-weight: 700;
        font-size: 0.9375rem;
        letter-spacing: -0.01em;
    }

    /* ── Nav links ── */
    .nav-links {
        display: flex;
        align-items: center;
        gap: 0.125rem;
        flex: 1;
        justify-content: center;
    }

    .nav-link {
        padding: 0.375rem 0.875rem;
        border-radius: var(--radius-md);
        font-size: 0.8125rem;
        font-weight: 500;
        color: var(--color-text-secondary);
        text-decoration: none;
        transition:
            background var(--transition-fast),
            color var(--transition-fast);
        position: relative;
    }
    .nav-link:hover {
        background: rgba(255, 255, 255, 0.05);
        color: var(--color-text-primary);
        text-decoration: none;
    }
    .nav-link.active {
        color: var(--color-text-primary);
        background: rgba(124, 58, 237, 0.12);
    }
    .nav-link.active::after {
        content: "";
        position: absolute;
        bottom: -0.375rem;
        left: 25%;
        right: 25%;
        height: 2px;
        background: var(--color-brand);
        border-radius: 1px;
    }

    /* ── Right actions ── */
    .nav-actions {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        flex-shrink: 0;
    }

    .icon-btn {
        width: 32px;
        height: 32px;
        border-radius: var(--radius-sm);
        border: none;
        background: transparent;
        color: var(--color-text-secondary);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition:
            color var(--transition-fast),
            background var(--transition-fast);
    }
    .icon-btn:hover {
        color: var(--color-text-primary);
        background: rgba(255, 255, 255, 0.06);
    }

    .user-avatar-btn {
        cursor: pointer;
        border: 0;
        padding: 0;
        background: transparent;
        border-radius: 50%;
        overflow: hidden;
        flex-shrink: 0;
        display: flex;
    }
    .user-avatar-btn:focus-visible {
        outline: none;
        box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.4);
    }

    .avatar-fallback {
        width: 30px;
        height: 30px;
        border-radius: 50%;
        background: var(--color-brand);
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.75rem;
        font-weight: 700;
    }
</style>
