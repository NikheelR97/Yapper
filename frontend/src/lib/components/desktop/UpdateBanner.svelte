<script lang="ts">
    import { updateAvailable, installUpdate } from "$lib/desktop/updater.js";

    let installing = false;

    async function handleRestart() {
        installing = true;
        await installUpdate();
        installing = false;
    }
</script>

{#if $updateAvailable}
    <div class="update-banner">
        <div class="banner-content">
            <span class="icon">🚀</span>
            <span class="text">
                <strong>Update available!</strong> Version {$updateAvailable} is ready.
            </span>
        </div>
        <button
            class="restart-btn"
            on:click={handleRestart}
            disabled={installing}
        >
            {installing ? "Installing..." : "Restart to Install"}
        </button>
    </div>
{/if}

<style>
    .update-banner {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 8px 16px;
        background: var(--bg-modifier-active, rgba(255, 255, 255, 0.05));
        border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.1));
        color: var(--text-normal, #dbdee1);
        font-size: 14px;
        flex-shrink: 0;
    }

    .banner-content {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .icon {
        font-size: 16px;
    }

    .text strong {
        color: var(--text-brand, #5865f2);
        font-weight: 600;
    }

    .restart-btn {
        background: var(--brand-experiment, #5865f2);
        color: white;
        border: none;
        border-radius: 4px;
        padding: 4px 12px;
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
        transition: background 0.2s ease;
    }

    .restart-btn:hover:not(:disabled) {
        background: var(--brand-experiment-500, #4752c4);
    }

    .restart-btn:disabled {
        opacity: 0.7;
        cursor: not-allowed;
    }
</style>
