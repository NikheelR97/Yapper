<!-- YapRecorder — audio message recorder.
  Flow:
   1. getUserMedia (audio only)
   2. MediaRecorder captures up to MAX_DURATION_S seconds
   3. On stop: encryptMedia -> request upload URL -> PUT to R2
   4. On success: embed media payload in Signal payload and send via onSend callback
-->

<script lang="ts">
	import { onDestroy } from "svelte";
	import { encryptMedia } from "$lib/signal/mediaEncrypt.js";
	import { api } from "$lib/api/client.js";

	/** Called with the Signal-ready payload JSON string when upload completes. */
	export let onSend: (mediaPayload: string) => void;
	export let onCancel: () => void;

	// ── Constants ──────────────────────────────────────────────────────────────
	const MAX_DURATION_S = 60;
	const PREFERRED_MIME = "audio/webm;codecs=opus";
	const FALLBACK_MIME = "audio/webm";
	const UPLOAD_URL_PATH = "/api/v1/media/upload-url";

	// ── State ──────────────────────────────────────────────────────────────────
	let recording = false;
	let uploading = false;
	let error: string | null = null;
	let elapsed = 0; // seconds

	let mediaRecorder: MediaRecorder | null = null;
	let stream: MediaStream | null = null;
	let stopTimer: ReturnType<typeof setTimeout> | null = null;
	let tickTimer: ReturnType<typeof setInterval> | null = null;
	let chunks: BlobPart[] = [];

	// ── Web Audio analyser for waveform visualisation ─────────────────────────
	let canvasEl: HTMLCanvasElement;
	let animFrame: number;
	let analyser: AnalyserNode | null = null;

	function startWaveform(srcStream: MediaStream): void {
		const ctx = new AudioContext();
		const src = ctx.createMediaStreamSource(srcStream);
		analyser = ctx.createAnalyser();
		analyser.fftSize = 64;
		src.connect(analyser);
		drawWaveform();
	}

	function drawWaveform(): void {
		if (!analyser || !canvasEl) return;
		const data = new Uint8Array(analyser.frequencyBinCount);
		analyser.getByteFrequencyData(data);

		const ctx = canvasEl.getContext("2d");
		if (!ctx) return;
		ctx.clearRect(0, 0, canvasEl.width, canvasEl.height);

		const barW = canvasEl.width / data.length;
		const midY = canvasEl.height / 2;
		ctx.fillStyle = "#a78bfa"; // violet-400

		for (let i = 0; i < data.length; i++) {
			const barH = (data[i] / 255) * canvasEl.height * 0.9;
			ctx.fillRect(i * barW, midY - barH / 2, barW - 1, barH);
		}

		animFrame = requestAnimationFrame(drawWaveform);
	}

	// ── Recording lifecycle ───────────────────────────────────────────────────
	async function startRecording(): Promise<void> {
		error = null;
		chunks = [];

		try {
			stream = await navigator.mediaDevices.getUserMedia({ audio: true });
		} catch {
			error = "Microphone access denied.";
			return;
		}

		const mimeType = MediaRecorder.isTypeSupported(PREFERRED_MIME)
			? PREFERRED_MIME
			: FALLBACK_MIME;

		mediaRecorder = new MediaRecorder(stream, { mimeType });
		mediaRecorder.ondataavailable = (e) => {
			if (e.data.size > 0) chunks.push(e.data);
		};
		mediaRecorder.onstop = () => handleStop(mimeType);

		mediaRecorder.start(250); // collect every 250 ms
		recording = true;
		startWaveform(stream);

		// Auto-stop at maximum duration
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
		cancelAnimationFrame(animFrame);
		recording = false;
		mediaRecorder.stop();
		stream?.getTracks().forEach((t) => t.stop());
	}

	async function handleStop(mimeType: string): Promise<void> {
		uploading = true;
		error = null;

		try {
			const blob = new Blob(chunks, { type: mimeType });
			const { encrypted, key, iv } = await encryptMedia(blob);

			// Request a pre-signed R2 PUT URL
			const { upload_url, object_key } = await api.post<{
				upload_url: string;
				object_key: string;
			}>(UPLOAD_URL_PATH, { media_type: "yap" });

			// Upload encrypted blob directly to R2
			const putResp = await fetch(upload_url, {
				method: "PUT",
				headers: { "Content-Type": "application/octet-stream" },
				body: encrypted,
			});

			if (!putResp.ok) {
				throw new Error(`R2 upload failed: ${putResp.status}`);
			}

			// Produce the media payload to be embedded in Signal ciphertext
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
			chunks = [];
		}
	}

	// ── Cleanup ───────────────────────────────────────────────────────────────
	onDestroy(() => {
		clearTimeout(stopTimer ?? undefined);
		clearInterval(tickTimer ?? undefined);
		cancelAnimationFrame(animFrame);
		stream?.getTracks().forEach((t) => t.stop());
	});

	// ── Helpers ───────────────────────────────────────────────────────────────
	function fmt(s: number): string {
		return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
	}
</script>

<div class="yap-recorder">
	{#if error}
		<p class="yap-error">{error}</p>
	{/if}

	<canvas bind:this={canvasEl} class="yap-wave" width="220" height="48"
	></canvas>

	<div class="yap-controls">
		{#if !recording && !uploading}
			<button
				class="yap-btn yap-btn--record"
				on:click={startRecording}
				aria-label="Start recording"
			>
				🎙
			</button>
			<button
				class="yap-btn yap-btn--cancel"
				on:click={onCancel}
				aria-label="Cancel">✕</button
			>
		{:else if recording}
			<span class="yap-timer">{fmt(elapsed)} / {fmt(MAX_DURATION_S)}</span
			>
			<button
				class="yap-btn yap-btn--stop"
				on:click={stopRecording}
				aria-label="Stop recording"
			>
				⏹
			</button>
		{:else}
			<span class="yap-uploading">Uploading…</span>
		{/if}
	</div>
</div>

<style>
	.yap-recorder {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		background: var(--surface-2, #1e1e2e);
		border-radius: 12px;
		width: 260px;
	}

	.yap-wave {
		border-radius: 6px;
		background: var(--surface-3, #2a2a3e);
		display: block;
		width: 100%;
	}

	.yap-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.yap-btn {
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

	.yap-btn:hover {
		opacity: 0.85;
	}
	.yap-btn--record {
		background: #a78bfa;
	}
	.yap-btn--stop {
		background: #f87171;
	}
	.yap-btn--cancel {
		background: var(--surface-3, #2a2a3e);
		color: #aaa;
	}

	.yap-timer,
	.yap-uploading {
		font-size: 13px;
		color: var(--text-muted, #888);
		font-variant-numeric: tabular-nums;
	}

	.yap-error {
		font-size: 12px;
		color: #f87171;
		margin: 0;
	}
</style>
