/**
 * WebSocket store — manages the persistent WS connection to the Yapper backend.
 *
 * Features:
 *  - Auth via first frame (token from authStore)
 *  - Exponential backoff reconnect (1s → 2s → 4s → … → 30s cap)
 *  - Routes inbound `Message { payload }` frames to registered handlers
 *  - Keepalive ping every 30 seconds
 */

import { get, writable } from 'svelte/store';
import { goto } from '$app/navigation';
import { authStore, clearAuth, refreshAccessToken } from '$stores/auth.js';
import { WS_URL } from '$lib/env.js';

type MessageHandler = (payload: unknown) => void;

type KeyDistPayload = {
	channel_id: string;
	from_user: string;
	from_device_id?: string | null;
	ciphertext: string;
	ek_public: string;
};

type KeyDistRequestPayload = {
	channel_id: string;
	requester_user_id: string;
	requester_device_id: string;
};

interface WsState {
	connected: boolean;
	error: string | null;
}

export const wsStore = writable<WsState>({ connected: false, error: null });

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let pingTimer: ReturnType<typeof setInterval> | null = null;
let proactiveReauthTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;
let stopped = false;

/** JWT lifetime is 15 min. Proactively re-auth at 12 min to avoid the
 *  server's 60-second re_auth_required degradation window. */
const PROACTIVE_REAUTH_MS = 12 * 60 * 1000;

const handlers = new Map<string, Set<MessageHandler>>();

// Bounded queue for messages sent while the WS is reconnecting.
// Flushed on next successful 'ready' event.
const MAX_PENDING = 50;
const pendingQueue: Record<string, unknown>[] = [];
// Idempotent message types where only the latest per-type matters.
const DEDUP_TYPES = new Set(['typing_start', 'read']);

// Always-on handler: decrypt and store incoming SenderKey distributions.
// Lazy-imported to keep @noble/curves (~150KB) and @noble/hashes (~50KB)
// out of the main entry chunk.
onWsMessage('key_dist', (payload) => {
	import('$lib/signal/index.js').then(({ receiveSenderKeyDist }) =>
		receiveSenderKeyDist(payload as KeyDistPayload)
	).catch((err) => console.error('[signal] Failed to process key_dist:', err));
});

onWsMessage('key_dist_v2', (payload) => {
	import('$lib/signal/index.js').then(({ receiveSenderKeyDist }) =>
		receiveSenderKeyDist(payload as KeyDistPayload)
	).catch((err) => console.error('[signal] Failed to process key_dist_v2:', err));
});

// When a new device joins a channel, redistribute our sender key to it.
onWsMessage('key_dist_request', (payload) => {
	import('$lib/signal/index.js').then(({ handleKeyDistRequest }) =>
		handleKeyDistRequest(payload as KeyDistRequestPayload)
	).catch((err) => console.error('[signal] Failed to handle key_dist_request:', err));
});

/** Register a handler for a specific message type inside `payload.type`. */
export function onWsMessage(type: string, handler: MessageHandler): () => void {
	if (!handlers.has(type)) handlers.set(type, new Set());
	handlers.get(type)!.add(handler);
	return () => handlers.get(type)?.delete(handler);
}

/** Send a raw JSON frame over the WebSocket.
 *  If the socket is not open, queues the message (up to MAX_PENDING) for
 *  delivery on the next successful reconnect. */
export function wsSend(msg: Record<string, unknown>): boolean {
	if (socket?.readyState === WebSocket.OPEN) {
		socket.send(JSON.stringify(msg));
		return true;
	}
	// Queue for delivery after reconnect.
	// For idempotent types (typing, read receipts), replace any existing
	// queued message of the same type — only the latest matters.
	const msgType = msg.type as string | undefined;
	if (msgType && DEDUP_TYPES.has(msgType)) {
		const idx = pendingQueue.findIndex((m) => m.type === msgType);
		if (idx !== -1) {
			pendingQueue[idx] = msg;
		} else if (pendingQueue.length < MAX_PENDING) {
			pendingQueue.push(msg);
		}
	} else if (pendingQueue.length < MAX_PENDING) {
		pendingQueue.push(msg);
	}
	return false;
}

/** Send a channel message via WebSocket. ciphertext must be base64(sig_64 || aes_ct). */
export function sendChannelMessage(
	channelId: string,
	wireCiphertext: string,
	iteration: number,
	messageType = 'text'
): boolean {
	return wsSend({
		type: 'send_channel',
		channel_id: channelId,
		ciphertext: wireCiphertext,
		message_type: messageType,
		msg_num: iteration,
	});
}

/** Send a DM message via WebSocket. */
export function sendDmMessage(
	conversationId: string,
	ciphertext: string,
	msgNum: number,
	ephemeralKey?: string,
	opkId?: number
): boolean {
	return wsSend({
		type: 'send_dm',
		conversation_id: conversationId,
		ciphertext,
		ephemeral_key: ephemeralKey ?? null,
		opk_id: opkId ?? null,
		msg_num: msgNum,
	});
}

/** Connect to the WebSocket server. Call after authentication. */
export function wsConnect(): void {
	stopped = false;
	if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
		return;
	}
	doConnect();
}

/** Disconnect and stop reconnection. Call on logout. */
export function wsDisconnect(): void {
	stopped = true;
	clearTimeout(reconnectTimer ?? undefined);
	clearInterval(pingTimer ?? undefined);
	socket?.close(1000, 'logout');
	socket = null;
	wsStore.set({ connected: false, error: null });
}

function doConnect(): void {
	if (stopped) return;
	const { accessToken } = get(authStore);
	if (!accessToken) return;

	const ws = new WebSocket(WS_URL);
	socket = ws;

	ws.onopen = () => {
		// Authenticate immediately — token NOT in query string (log safety)
		ws.send(JSON.stringify({ type: 'auth', token: get(authStore).accessToken }));
	};

	ws.onmessage = (event) => {
		let frame: { type: string;[k: string]: unknown };
		try {
			frame = JSON.parse(event.data as string);
		} catch {
			return;
		}

		switch (frame.type) {
			case 'ready':
				wsStore.set({ connected: true, error: null });
				reconnectDelay = 1000;
				startPing(ws);
				scheduleProactiveReauth(ws);
				// Flush any messages queued during reconnection
				while (pendingQueue.length > 0) {
					const queued = pendingQueue.shift()!;
					ws.send(JSON.stringify(queued));
				}
				// Notify reconnect listeners so stale stores can refresh
				handlers.get('_reconnected')?.forEach((h) => h({}));
				break;

			case 'message': {
				const payload = frame.payload as { type?: string } | undefined;
				if (payload?.type) {
					handlers.get(payload.type)?.forEach((h) => h(payload));
				}
				break;
			}

			case 're_auth_required': {
				// Server is about to expire our session — refresh and re-send
				refreshAccessToken().then((freshToken) => {
					if (freshToken && ws.readyState === WebSocket.OPEN) {
						ws.send(JSON.stringify({ type: 'reauth', token: freshToken }));
					}
				});
				break;
			}

			case 'typing':
				handlers.get('typing')?.forEach((h) => h(frame));
				break;

			case 'typing_stop':
				handlers.get('typing_stop')?.forEach((h) => h(frame));
				break;

			case 'read_receipt':
				handlers.get('read_receipt')?.forEach((h) => h(frame));
				break;

			case 'presence':
				handlers.get('presence')?.forEach((h) => h(frame));
				break;

			case 'canvas_update':
				handlers.get('canvas_update')?.forEach((h) => h(frame));
				break;

			case 'pong':
				break;

			case 'error':
				console.warn('[WS] Server error:', frame.code, frame.message);
				// Device revocation (4001) must be handled immediately at the WS layer,
				// not delegated to optional layout handlers which may not yet be mounted.
				if (frame.code === 4001) {
					wsDisconnect();
					clearAuth();
					void goto('/login');
					return;
				}
				handlers.get('ws_error')?.forEach((h) => h(frame));
				break;
		}
	};

	ws.onerror = () => {
		wsStore.update((s) => ({ ...s, error: 'Connection error' }));
	};

	ws.onclose = (event) => {
		clearInterval(pingTimer ?? undefined);
		clearTimeout(proactiveReauthTimer ?? undefined);
		wsStore.set({ connected: false, error: event.code !== 1000 ? 'disconnected' : null });
		socket = null;

		if (!stopped && event.code !== 1000) {
			// Reconnect with exponential backoff (cap at 30 s)
			reconnectTimer = setTimeout(() => {
				reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
				doConnect();
			}, reconnectDelay);
		}
	};
}

/** Notify the server that the current user is typing in a channel (throttle client-side). */
export function sendTypingStart(channelId: string): boolean {
	return wsSend({ type: 'typing_start', channel_id: channelId });
}

/** Notify the server that a message has been read. */
export function sendMarkRead(messageId: string, channelId: string): boolean {
	return wsSend({ type: 'read', message_id: messageId, channel_id: channelId });
}

function startPing(ws: WebSocket): void {
	clearInterval(pingTimer ?? undefined);
	pingTimer = setInterval(() => {
		if (ws.readyState === WebSocket.OPEN) {
			ws.send(JSON.stringify({ type: 'ping' }));
		}
	}, 30_000);
}

/** Refresh the JWT and re-auth proactively at 12 min to avoid the server's
 *  60-second re_auth_required degradation window (JWT lifetime = 15 min). */
function scheduleProactiveReauth(ws: WebSocket): void {
	clearTimeout(proactiveReauthTimer ?? undefined);
	proactiveReauthTimer = setTimeout(async () => {
		const freshToken = await refreshAccessToken();
		if (freshToken && ws.readyState === WebSocket.OPEN) {
			ws.send(JSON.stringify({ type: 'reauth', token: freshToken }));
		}
	}, PROACTIVE_REAUTH_MS);
}
