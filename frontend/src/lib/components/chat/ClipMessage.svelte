<script lang="ts">
    import { decryptMedia } from "$lib/signal/mediaEncrypt.js";

    export let payload: {
        object_key: string;
        key: string;
        iv: string;
        mime_type: string;
    };

    const R2_BASE = import.meta.env.VITE_R2_PUBLIC_URL ?? "";

    let videoEl: HTMLVideoElement;
    let blobUrl: string | null = null;
    let loading = false;
    let error: string | null = null;

    async function loadAndPlay(): Promise<void> {
        if (blobUrl) {
            videoEl.paused ? videoEl.play() : videoEl.pause();
            return;
        }

        loading = true;
        error = null;

        try {
            const r2Url = `${R2_BASE}/${payload.object_key}`;
            const resp = await fetch(r2Url);
            if (!resp.ok) throw new Error(`Fetch failed: ${resp.status}`);

            const encryptedBuf = await resp.arrayBuffer();
            const blob = await decryptMedia(
                encryptedBuf,
                payload.key,
                payload.iv,
                payload.mime_type,
            );

            blobUrl = URL.createObjectURL(blob);
            await new Promise((r) => requestAnimationFrame(r));
            videoEl.src = blobUrl;
            await videoEl.play();
        } catch (e) {
            error = "Could not play Clip.";
            console.error("[ClipMessage] decrypt/play error:", e);
        } finally {
            loading = false;
        }
    }
</script>

/** * ClipMessage — playback UI for a received, E2EE video (Clip) message. * *
Decryption is deferred to first play (lazy), same pattern as YapMessage. */

<div class="clip-msg">
    <!-- svelte-ignore a11y-media-has-caption -->
    <video
        bind:this={videoEl}
        class="clip-video"
        controls={!!blobUrl}
        playsinline
    ></video>

    {#if !blobUrl}
        <button
            class="clip-play-overlay"
            on:click={loadAndPlay}
            disabled={loading}
            aria-label="Play Clip"
        >
            {#if loading}
                <span class="clip-spinner"></span>
            {:else}
                <span class="clip-play-icon">▶</span>
            {/if}
        </button>
    {/if}

    {#if error}
        <p class="clip-error">{error}</p>
    {/if}
</div>

<style>
    .clip-msg {
        position: relative;
        border-radius: 12px;
        overflow: hidden;
        max-width: 320px;
        background: #000;
    }

    .clip-video {
        display: block;
        width: 100%;
        aspect-ratio: 16 / 9;
        object-fit: cover;
    }

    .clip-play-overlay {
        position: absolute;
        inset: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        background: rgba(0, 0, 0, 0.45);
        border: none;
        cursor: pointer;
        transition: background 0.15s;
    }

    .clip-play-overlay:hover {
        background: rgba(0, 0, 0, 0.6);
    }
    .clip-play-overlay:disabled {
        cursor: not-allowed;
        opacity: 0.7;
    }

    .clip-play-icon {
        font-size: 36px;
        color: #fff;
        text-shadow: 0 2px 8px rgba(0, 0, 0, 0.6);
    }

    .clip-spinner {
        display: inline-block;
        width: 28px;
        height: 28px;
        border: 3px solid rgba(255, 255, 255, 0.3);
        border-top-color: #fff;
        border-radius: 50%;
        animation: spin 0.7s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .clip-error {
        position: absolute;
        bottom: 8px;
        left: 8px;
        font-size: 12px;
        color: #f87171;
        margin: 0;
    }
</style>
