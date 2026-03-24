<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import type { Message } from "$stores/conversations.js";
  import type { ServerEmoji } from "$stores/servers.js";
  import { authStore } from "$stores/auth.js";
  import { get } from "svelte/store";
  import { sendMarkRead } from "$stores/ws.js";
  import YapMessage from "./YapMessage.svelte";
  import ClipMessage from "./ClipMessage.svelte";
  import ReadReceipt from "./ReadReceipt.svelte";
  import { renderMessageTokens } from "./renderMessageText.js";

  export let messages: Message[];
  /** Needed for sendMarkRead events. */
  export let channelId: string;
  /** 'dm' | 'channel' — controls ReadReceipt display mode. */
  export let mode: "dm" | "channel" = "channel";
  /** Custom server emojis for :shortcode: rendering. */
  export let serverEmojis: ServerEmoji[] = [];

  const myId = get(authStore).user?.id ?? "";

  // ── Emoji shortcode rendering ─────────────────────────────────────────────

  $: emojiMap = new Map(serverEmojis.map((e) => [e.name, e.imageUrl]));

  // ── Memoized token rendering ──────────────────────────────────────────────
  // Cache rendered tokens by message id + text to avoid re-parsing on every
  // reactive update (typing indicators, presence changes, etc).
  const tokenCache = new Map<string, ReturnType<typeof renderMessageTokens>>();

  function getTokens(msg: Message) {
    if (!msg.text) return [];
    const cacheKey = msg.id;
    const cached = tokenCache.get(cacheKey);
    if (cached) return cached;
    const tokens = renderMessageTokens(msg.text, emojiMap);
    tokenCache.set(cacheKey, tokens);
    return tokens;
  }

  // Invalidate cache when emoji map changes (server emoji add/remove)
  $: if (emojiMap) tokenCache.clear();

  function formatTime(iso: string): string {
    return new Date(iso).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  // ── Media payload parsing ─────────────────────────────────────────────────

  function parseMediaPayload(text: string | null): {
    object_key: string;
    key: string;
    iv: string;
    mime_type: string;
  } | null {
    if (!text) return null;
    try {
      const p = JSON.parse(text);
      if (p.object_key && p.key && p.iv && p.mime_type) return p;
    } catch {
      /* not a media payload */
    }
    return null;
  }

  // ── Viewport windowing ────────────────────────────────────────────────────
  // Only render messages near the viewport to cap DOM nodes at ~80 instead
  // of 200+. Uses a generous buffer so scrolling feels seamless.

  const RENDER_WINDOW = 60; // messages to render around viewport
  const SCROLL_BUFFER = 20; // extra messages above/below visible area
  let renderStart = 0;
  let renderEnd = 0;
  let userScrolledUp = false;

  $: {
    // When messages change, recompute the visible window.
    // Default: show the most recent messages (bottom of list).
    const total = messages.length;
    if (total <= RENDER_WINDOW) {
      renderStart = 0;
      renderEnd = total;
    } else if (!userScrolledUp) {
      // User is at the bottom — render the last RENDER_WINDOW messages
      renderStart = total - RENDER_WINDOW;
      renderEnd = total;
    }
    // If user has scrolled up, renderStart/renderEnd are managed by onScroll
  }

  $: visibleMessages = messages.slice(renderStart, renderEnd);
  $: topSpacer = renderStart; // number of messages above the window

  function onScroll() {
    if (!listEl) return;
    const { scrollTop, scrollHeight, clientHeight } = listEl;
    const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

    userScrolledUp = distanceFromBottom > 100;

    if (messages.length <= RENDER_WINDOW) return;

    // Estimate which messages are near the viewport based on scroll ratio
    const scrollRatio = scrollTop / Math.max(1, scrollHeight - clientHeight);
    const centerIndex = Math.floor(scrollRatio * messages.length);
    const halfWindow = Math.floor(RENDER_WINDOW / 2);

    renderStart = Math.max(0, centerIndex - halfWindow - SCROLL_BUFFER);
    renderEnd = Math.min(messages.length, centerIndex + halfWindow + SCROLL_BUFFER);
  }

  // ── IntersectionObserver for read receipts ───────────────────────────────

  let listEl: HTMLDivElement;
  let observer: IntersectionObserver;

  onMount(() => {
    observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const msgId = (entry.target as HTMLElement).dataset.msgId;
          if (msgId) {
            sendMarkRead(msgId, channelId);
            observer.unobserve(entry.target); // only fire once per message
          }
        }
      },
      { root: listEl, threshold: 0.5 },
    );
  });

  onDestroy(() => observer?.disconnect());

  /** Svelte action: attach observer when a message element is mounted. */
  function observe(node: HTMLElement, msgId: string) {
    node.dataset.msgId = msgId;
    observer?.observe(node);
    return {
      destroy() {
        observer?.unobserve(node);
      },
    };
  }

  // ── Auto-scroll to bottom on new messages ─────────────────────────────────
  let prevLength = 0;
  $: if (messages.length > prevLength && !userScrolledUp) {
    prevLength = messages.length;
    tick().then(() => {
      if (listEl) listEl.scrollTop = listEl.scrollHeight;
    });
  }
</script>

<div
  class="message-list"
  bind:this={listEl}
  on:scroll={onScroll}
  role="log"
  aria-live="polite"
  aria-label="Messages"
>
  {#if topSpacer > 0}
    <div class="spacer" aria-hidden="true" />
  {/if}

  {#each visibleMessages as msg (msg.id)}
    {@const isOwn = msg.senderId === myId}
    {@const mediaPayload = parseMediaPayload(msg.text)}
    {@const renderedTokens = getTokens(msg)}

    <div class="message" class:own={isOwn} use:observe={msg.id}>
      {#if msg.decryptError}
        <span class="bubble error" title="Decryption failed"
          >🔒 Unable to decrypt</span
        >
      {:else if msg.messageType === "yap" && mediaPayload}
        <YapMessage payload={mediaPayload} />
      {:else if msg.messageType === "clip" && mediaPayload}
        <ClipMessage payload={mediaPayload} />
      {:else if msg.text === null}
        <span class="bubble loading">…</span>
      {:else}
        <span class="bubble">
          {#each renderedTokens as token, index (`${msg.id}:${index}`)}
            {#if token.type === "emoji"}
              <img
                src={token.url}
                alt={`:${token.name}:`}
                title={`:${token.name}:`}
                class="inline-emoji"
                loading="lazy"
              />
            {:else}
              {token.value}
            {/if}
          {/each}
        </span>
      {/if}

      <div class="meta">
        <time class="timestamp" datetime={msg.createdAt}
          >{formatTime(msg.createdAt)}</time
        >
        {#if isOwn}
          <ReadReceipt messageId={msg.id} {mode} />
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .message-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
    overflow-y: auto;
    flex: 1;
  }

  .spacer {
    /* Estimated height for messages above the render window.
       Not pixel-perfect, but prevents the scrollbar from jumping drastically. */
    flex-shrink: 0;
    min-height: 1px;
  }

  .message {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.125rem;
  }

  .message.own {
    align-items: flex-end;
  }

  .bubble {
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
    border-radius: 1rem 1rem 1rem 0.25rem;
    color: var(--color-text-primary);
    font-size: 0.9375rem;
    line-height: 1.45;
    max-width: 70%;
    padding: 0.5rem 0.875rem;
    word-break: break-word;
  }

  .message.own .bubble {
    background: var(--color-brand);
    border-color: transparent;
    border-radius: 1rem 1rem 0.25rem 1rem;
    color: white;
  }

  .bubble.error {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.3);
    color: #fca5a5;
    font-size: 0.8125rem;
  }

  .bubble.loading {
    color: var(--color-text-muted);
  }

  .inline-emoji {
    width: 20px;
    height: 20px;
    vertical-align: -0.35em;
    object-fit: contain;
    border-radius: 2px;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .timestamp {
    color: var(--color-text-muted);
    font-size: 0.6875rem;
    padding: 0 0.25rem;
  }
</style>
