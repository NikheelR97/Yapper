<script lang="ts">
    import { onDestroy } from "svelte";
    import { encryptMedia } from "$lib/signal/mediaEncrypt.js";
    import { api } from "$lib/api/client.js";

    export let onSend: (mediaPayload: string) => void;
    export let onCancel: () => void;

    // ── Constants ──────────────────────────────────────────────────────────────
    const MAX_DURATION_S = 30;
    const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10 MB (free tier)
    // Prefer VP9 in WebM; fall back to whatever the browser supports.
    const PREFERRED_MIMES = [
        "video/webm;codecs=vp9,opus",
        "video/webm;codecs=vp8,opus",
        "video/webm",
    ];
    const UPLOAD_URL_PATH = "/api/v1/media/upload-url";

    // ── State ──────────────────────────────────────────────────────────────────
    let recording = false;
    let uploading = false;
    let uploadProgress = 0; // 0–100
    let error: string | null = null;
    let elapsed = 0;

    let videoEl: HTMLVideoElement;
    let mediaRecorder: MediaRecorder | null = null;
    let stream: MediaStream | null = null;
    let stopTimer: ReturnType<typeof setTimeout> | null = null;
    let tickTimer: ReturnType<typeof setInterval> | null = null;
    let chunks: BlobPart[] = [];

    // ── Helpers ────────────────────────────────────────────────────────────────
    function pickMimeType(): string {
        for (const mime of PREFERRED_MIMES) {
            if (MediaRecorder.isTypeSupported(mime)) return mime;
        }
        return "";
    }

    function fmt(s: number): string {
        return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
    }

    // ── Recording lifecycle ───────────────────────────────────────────────────
    async function startRecording(): Promise<void> {
        error = null;
        chunks = [];

        try {
            stream = await navigator.mediaDevices.getUserMedia({
                video: true,
                audio: true,
            });
        } catch {
            error = "Camera or microphone access denied.";
            return;
        }

        // Show live preview
        videoEl.srcObject = stream;
        videoEl.muted = true;
        await videoEl.play();

        const mimeType = pickMimeType();
        const opts = mimeType ? { mimeType } : {};

        mediaRecorder = new MediaRecorder(stream, opts);
        mediaRecorder.ondataavailable = (e) => {
            if (e.data.size > 0) chunks.push(e.data);
        };
        mediaRecorder.onstop = () => handleStop(mimeType || "video/webm");

        mediaRecorder.start(250);
        recording = true;

        stopTimer = setTimeout(() => stopRecording(), MAX_DURATION_S * 1000);
        tickTimer = setInterval(() => {
            elapsed += 1;
            if (elapsed >= MAX_DURATION_S) stopRecording();
        }, 1000);
    }

    function stopRecording(): void {
        if (!recording || !mediaRecorder) return;
        clearTimeout(stopTimer ?? undefined);
        clearInterval(tickTimer ?? undefined);
        recording = false;
        mediaRecorder.stop();
        // Keep stream alive for the video element until we clean up
    }

    /** Upload encrypted blob via XHR (supports progress events). */
    function uploadToR2(url: string, body: ArrayBuffer): Promise<void> {
        return new Promise((resolve, reject) => {
            const xhr = new XMLHttpRequest();
            xhr.open("PUT", url);
            xhr.setRequestHeader("Content-Type", "application/octet-stream");
            xhr.upload.onprogress = (e) => {
                if (e.lengthComputable) {
                    uploadProgress = Math.round((e.loaded / e.total) * 100);
                }
            };
            xhr.onload = () =>
                xhr.status >= 200 && xhr.status < 300
                    ? resolve()
                    : reject(new Error(`R2 upload failed: ${xhr.status}`));
            xhr.onerror = () => reject(new Error("Upload network error"));
            xhr.send(body);
        });
    }

    async function handleStop(mimeType: string): Promise<void> {
        // Stop preview stream
        stream?.getTracks().forEach((t) => t.stop());
        videoEl.srcObject = null;

        uploading = true;
        uploadProgress = 0;
        error = null;

        try {
            const blob = new Blob(chunks, { type: mimeType });

            // Client-side size check before encrypting (10 MB free tier)
            if (blob.size > MAX_FILE_SIZE) {
                throw new Error(
                    `Recording too large (${(blob.size / 1024 / 1024).toFixed(1)} MB). Max ${MAX_FILE_SIZE / 1024 / 1024} MB.`,
                );
            }

            const { encrypted, key, iv } = await encryptMedia(blob);

            const { upload_url, object_key } = await api.post<{
                upload_url: string;
                object_key: string;
            }>(UPLOAD_URL_PATH, {
                media_type: "clip",
                content_length: encrypted.byteLength,
            });

            // Upload with progress
            await uploadToR2(upload_url, encrypted);

            const mediaPayload = JSON.stringify({
                object_key,
                key,
                iv,
                mime_type: mimeType,
            });
            onSend(mediaPayload);
        } catch (e) {
            error = e instanceof Error ? e.message : "Upload failed";
        } finally {
            uploading = false;
            uploadProgress = 0;
            chunks = [];
        }
    }

    // ── Cleanup ───────────────────────────────────────────────────────────────
    onDestroy(() => {
        clearTimeout(stopTimer ?? undefined);
        clearInterval(tickTimer ?? undefined);
        stream?.getTracks().forEach((t) => t.stop());
    });
</script>

<!-- ClipRecorder — video+audio message recorder.
  Flow: getUserMedia -> MediaRecorder (30s cap) -> encryptMedia -> upload to R2 -> Signal payload via onSend
-->

<div class="clip-recorder">
    <!-- Live camera preview -->
    <video bind:this={videoEl} class="clip-preview" playsinline muted>
        <track kind="captions" />
    </video>

    {#if error}
        <p class="clip-error">{error}</p>
    {/if}

    <div class="clip-controls">
        {#if !recording && !uploading}
            <button
                class="clip-btn clip-btn--record"
                on:click={startRecording}
                aria-label="Start recording"
            >
                🎬
            </button>
            <button
                class="clip-btn clip-btn--cancel"
                on:click={onCancel}
                aria-label="Cancel">✕</button
            >
        {:else if recording}
            <span class="clip-rec-dot"></span>
            <span class="clip-timer"
                >{fmt(elapsed)} / {fmt(MAX_DURATION_S)}</span
            >
            <button
                class="clip-btn clip-btn--stop"
                on:click={stopRecording}
                aria-label="Stop recording"
            >
                ⏹
            </button>
        {:else}
            <span class="clip-uploading">Uploading… {uploadProgress}%</span>
            <div class="clip-progress-bar">
                <div
                    class="clip-progress-fill"
                    style="width: {uploadProgress}%"
                ></div>
            </div>
        {/if}
    </div>
</div>

<style>
    .clip-recorder {
        display: flex;
        flex-direction: column;
        gap: 8px;
        background: var(--surface-2, #1e1e2e);
        border-radius: 12px;
        overflow: hidden;
        width: 280px;
    }

    .clip-preview {
        width: 100%;
        aspect-ratio: 16 / 9;
        object-fit: cover;
        background: #000;
    }

    .clip-controls {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 8px 10px;
    }

    .clip-btn {
        width: 36px;
        height: 36px;
        border-radius: 50%;
        border: none;
        cursor: pointer;
        font-size: 16px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: opacity 0.15s;
    }

    .clip-btn:hover {
        opacity: 0.85;
    }
    .clip-btn--record {
        background: #a78bfa;
    }
    .clip-btn--stop {
        background: #f87171;
    }
    .clip-btn--cancel {
        background: var(--surface-3, #2a2a3e);
        color: #aaa;
    }

    .clip-rec-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: #f87171;
        animation: blink 1s step-end infinite;
        flex-shrink: 0;
    }

    @keyframes blink {
        50% {
            opacity: 0;
        }
    }

    .clip-timer,
    .clip-uploading {
        font-size: 13px;
        color: var(--text-muted, #888);
        font-variant-numeric: tabular-nums;
    }

    .clip-progress-bar {
        flex: 1;
        height: 4px;
        background: var(--surface-3, #2a2a3e);
        border-radius: 2px;
        overflow: hidden;
    }

    .clip-progress-fill {
        height: 100%;
        background: #a78bfa;
        border-radius: 2px;
        transition: width 0.2s ease;
    }

    .clip-error {
        font-size: 12px;
        color: #f87171;
        margin: 0;
        padding: 0 10px;
    }
</style>
