<script lang="ts">
    import { decryptMedia } from "$lib/signal/mediaEncrypt.js";

    /** Signal-decrypted JSON payload: { object_key, key, iv, mime_type } */
    export let payload: {
        object_key: string;
        key: string;
        iv: string;
        mime_type: string;
    };

    // R2 public base URL — served via Cloudflare CDN
    const R2_BASE = import.meta.env.VITE_R2_PUBLIC_URL ?? "";

    let audioEl: HTMLAudioElement;
    let blobUrl: string | null = null;
    let loading = false;
    let error: string | null = null;
    let duration = 0;
    let currentTime = 0;
    let playing = false;

    async function loadAndPlay(): Promise<void> {
        if (blobUrl) {
            // Already decrypted — just toggle play/pause
            playing ? audioEl?.pause() : audioEl?.play();
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

            // Play after giving the audio element a tick to load
            await new Promise((r) => requestAnimationFrame(r));
            audioEl.src = blobUrl;
            await audioEl.play();
        } catch (e) {
            error = "Could not play Yap.";
            console.error("[YapMessage] decrypt/play error:", e);
        } finally {
            loading = false;
        }
    }

    function onTimeUpdate(): void {
        currentTime = audioEl?.currentTime ?? 0;
    }

    function onLoadedMetadata(): void {
        duration = isFinite(audioEl?.duration) ? audioEl.duration : 0;
    }

    function onPlay(): void {
        playing = true;
    }
    function onPause(): void {
        playing = false;
    }
    function onEnded(): void {
        playing = false;
        currentTime = 0;
    }

    function fmt(s: number): string {
        const safe = isFinite(s) ? s : 0;
        return `${Math.floor(safe / 60)}:${String(Math.floor(safe % 60)).padStart(2, "0")}`;
    }

    function progress(): number {
        return duration > 0 ? (currentTime / duration) * 100 : 0;
    }
</script>

/** * YapMessage — playback UI for a received, E2EE audio (Yap) message. * *
Decryption is deferred to first play (lazy) to avoid unnecessary network *
requests and CPU for messages that scroll past without being opened. */

<!-- Hidden audio element drives playback -->
<audio
    bind:this={audioEl}
    on:timeupdate={onTimeUpdate}
    on:loadedmetadata={onLoadedMetadata}
    on:play={onPlay}
    on:pause={onPause}
    on:ended={onEnded}
></audio>

<div class="yap-msg">
    <button
        class="yap-play"
        class:yap-play--playing={playing}
        on:click={loadAndPlay}
        disabled={loading}
        aria-label={playing ? "Pause" : "Play Yap"}
    >
        {#if loading}
            <span class="yap-spinner"></span>
        {:else}
            {playing ? "⏸" : "▶"}
        {/if}
    </button>

    <div class="yap-track">
        <div class="yap-bar">
            <div class="yap-fill" style="width: {progress()}%"></div>
        </div>
        <div class="yap-times">
            <span>{fmt(currentTime)}</span>
            {#if duration > 0}<span>{fmt(duration)}</span>{/if}
        </div>
    </div>

    {#if error}
        <span class="yap-err" title={error}>⚠</span>
    {/if}
</div>

<style>
    .yap-msg {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px 12px;
        background: var(--surface-2, #1e1e2e);
        border-radius: 12px;
        min-width: 180px;
        max-width: 280px;
    }

    .yap-play {
        width: 36px;
        height: 36px;
        border-radius: 50%;
        border: none;
        background: #a78bfa;
        color: #fff;
        cursor: pointer;
        font-size: 14px;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        transition: background 0.15s;
    }

    .yap-play:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    .yap-play--playing {
        background: #7c3aed;
    }

    .yap-track {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .yap-bar {
        height: 4px;
        border-radius: 2px;
        background: var(--surface-3, #2a2a3e);
        overflow: hidden;
    }

    .yap-fill {
        height: 100%;
        background: #a78bfa;
        border-radius: 2px;
        transition: width 0.1s linear;
    }

    .yap-times {
        display: flex;
        justify-content: space-between;
        font-size: 10px;
        color: var(--text-muted, #888);
        font-variant-numeric: tabular-nums;
    }

    .yap-spinner {
        display: inline-block;
        width: 14px;
        height: 14px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: #fff;
        border-radius: 50%;
        animation: spin 0.7s linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    .yap-err {
        font-size: 14px;
        cursor: help;
    }
</style>
