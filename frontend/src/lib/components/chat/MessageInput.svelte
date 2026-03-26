<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import {
		sendTypingStart,
	} from "$stores/ws.js";
	import { sendMessage as sendChannelMessageWithState } from "$stores/servers.js";
	import YapRecorder from "./YapRecorder.svelte";
	import ClipRecorder from "./ClipRecorder.svelte";
	import EmojiPicker from "$lib/components/emoji/EmojiPicker.svelte";

	export let disabled = false;
	export let placeholder = "Send a message…";
	/** When set, typing events + WS sends are scoped to this channel. */
	export let channelId: string | undefined = undefined;
	/** When set, messages go as DMs to this conversation. */
	export let conversationId: string | undefined = undefined;
	/** Recipient user ID — required for DMs to build the Signal session. */
	export let recipientId: string | undefined = undefined;
	/** Server ID — used by EmojiPicker to show custom server emojis. */
	export let serverId: string | undefined = undefined;

	type ActiveRecorder = "yap" | "clip" | null;

	let text = "";
	let typingThrottle: ReturnType<typeof setTimeout> | null = null;
	let activeRecorder: ActiveRecorder = null;
	let showEmojiPicker = false;
	let textareaEl: HTMLTextAreaElement;
	const dispatch = createEventDispatcher<{ send: string }>();

	function handleInput() {
		if (!channelId || typingThrottle) return;
		sendTypingStart(channelId);
		// Throttle to once per 2s — server auto-stops after 5s silence
		typingThrottle = setTimeout(() => {
			typingThrottle = null;
		}, 2000);
	}

	function handleSubmit() {
		const trimmed = text.trim();
		if (!trimmed || disabled) return;
		dispatch("send", trimmed);
		text = "";
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleSubmit();
		}
	}

	// ── Emoji picker ─────────────────────────────────────────────────────────

	function handleEmojiSelect(e: CustomEvent<string>) {
		const emoji = e.detail;
		const start = textareaEl?.selectionStart ?? text.length;
		const end = textareaEl?.selectionEnd ?? text.length;
		text = text.slice(0, start) + emoji + text.slice(end);
		showEmojiPicker = false;
		// Restore focus + move cursor after the inserted emoji
		setTimeout(() => {
			if (textareaEl) {
				textareaEl.focus();
				const pos = start + emoji.length;
				textareaEl.selectionStart = textareaEl.selectionEnd = pos;
			}
		}, 0);
	}

	// ── Media send helpers ────────────────────────────────────────────────────

	/**
	 * Called by YapRecorder / ClipRecorder with the raw media payload JSON.
	 * We re-encrypt it inside a Signal message (same path as text) and send.
	 */
	async function handleMediaSend(
		mediaPayloadJson: string,
		messageType: "yap" | "clip",
	) {
		activeRecorder = null;
		if (disabled) return;

		try {
			if (channelId) {
				await sendChannelMessageWithState(
					channelId,
					mediaPayloadJson,
					{ messageType },
				);
			} else if (conversationId && recipientId) {
				dispatch("send", mediaPayloadJson);
			}
		} catch (e) {
			console.error("[MessageInput] Failed to send media message:", e);
		}
	}
</script>

<!-- Emoji picker popup — shown above the input bar -->
{#if showEmojiPicker}
	<div class="emoji-popup">
		<EmojiPicker {serverId} on:select={handleEmojiSelect} />
	</div>
{/if}

<!-- Recorder overlays — shown inline above the input bar -->
{#if activeRecorder === "yap"}
	<div class="recorder-tray">
		<YapRecorder
			onSend={(p) => handleMediaSend(p, "yap")}
			onCancel={() => {
				activeRecorder = null;
			}}
		/>
	</div>
{:else if activeRecorder === "clip"}
	<div class="recorder-tray">
		<ClipRecorder
			onSend={(p) => handleMediaSend(p, "clip")}
			onCancel={() => {
				activeRecorder = null;
			}}
		/>
	</div>
{/if}

<form class="input-bar" on:submit|preventDefault={handleSubmit}>
	<!-- Yap (microphone) button -->
	<button
		type="button"
		class="media-btn"
		class:active={activeRecorder === "yap"}
		on:click={() => {
			activeRecorder = activeRecorder === "yap" ? null : "yap";
		}}
		{disabled}
		aria-label="Record a Yap"
		title="Record a Yap (voice message)"
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
			<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
			<path d="M19 10v2a7 7 0 0 1-14 0v-2" />
			<line x1="12" y1="19" x2="12" y2="23" />
			<line x1="8" y1="23" x2="16" y2="23" />
		</svg>
	</button>

	<!-- Clip (camera) button -->
	<button
		type="button"
		class="media-btn"
		class:active={activeRecorder === "clip"}
		on:click={() => {
			activeRecorder = activeRecorder === "clip" ? null : "clip";
		}}
		{disabled}
		aria-label="Record a Clip"
		title="Record a Clip (video message)"
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
			<polygon points="23 7 16 12 23 17 23 7" />
			<rect x="1" y="5" width="15" height="14" rx="2" ry="2" />
		</svg>
	</button>

	<!-- Emoji button -->
	<button
		type="button"
		class="media-btn"
		class:active={showEmojiPicker}
		on:click={() => (showEmojiPicker = !showEmojiPicker)}
		{disabled}
		aria-label="Emoji picker"
		title="Emoji picker"
	>
		😊
	</button>

	<textarea
		bind:value={text}
		bind:this={textareaEl}
		on:input={handleInput}
		on:keydown={handleKeydown}
		{placeholder}
		{disabled}
		rows={1}
		maxlength={4000}
		aria-label="Message"
		class="input"
	></textarea>

	<button
		type="submit"
		class="send-btn"
		disabled={disabled || !text.trim()}
		aria-label="Send"
	>
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			aria-hidden="true"
		>
			<line x1="22" y1="2" x2="11" y2="13"></line>
			<polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
		</svg>
	</button>
</form>

<style>
	.emoji-popup {
		position: absolute;
		bottom: calc(100% + 4px);
		left: 0.75rem;
		z-index: 200;
	}

	.recorder-tray {
		padding: 0.5rem 1rem;
		border-top: 1px solid var(--color-border);
		background: var(--color-bg-base);
	}

	.input-bar {
		position: relative;
		display: flex;
		align-items: flex-end;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border);
		background: var(--color-bg-base);
	}

	.media-btn {
		flex-shrink: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: 1px solid var(--color-border);
		border-radius: 50%;
		color: var(--color-text-muted);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			color var(--transition-fast),
			border-color var(--transition-fast);
	}

	.media-btn:hover:not(:disabled) {
		background: var(--color-bg-surface);
		color: var(--color-text-primary);
	}

	.media-btn.active {
		background: var(--color-brand);
		border-color: var(--color-brand);
		color: white;
	}

	.media-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.input {
		flex: 1;
		resize: none;
		background: var(--color-bg-surface);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-primary);
		font-size: 0.9375rem;
		line-height: 1.5;
		max-height: 160px;
		overflow-y: auto;
		padding: 0.5rem 0.75rem;
		transition: border-color var(--transition-fast);
	}

	.input:focus {
		border-color: var(--color-brand);
		outline: none;
	}

	.input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.send-btn {
		flex-shrink: 0;
		width: 2.25rem;
		height: 2.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-brand);
		border: none;
		border-radius: 50%;
		color: white;
		cursor: pointer;
		transition:
			background var(--transition-fast),
			opacity var(--transition-fast);
	}

	.send-btn:hover:not(:disabled) {
		background: var(--color-brand-dark);
	}

	.send-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
</style>
