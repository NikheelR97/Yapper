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
import { authStore } from '$stores/auth.js';
import { receiveSenderKeyDist } from '$lib/signal/index.js';

const WS_URL = (import.meta.env.VITE_API_URL ?? 'http://localhost:8080').replace(
	/^http/,
	'ws'
);

type MessageHandler = (payload: unknown) => void;

interface WsState {
	connected: boolean;
	error: string | null;
}

export const wsStore = writable<WsState>({ connected: false, error: null });

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let pingTimer: ReturnType<typeof setInterval> | null = null;
let reconnectDelay = 1000;
let stopped = false;

const handlers = new Map<string, Set<MessageHandler>>();

// Always-on handler: decrypt and store incoming SenderKey distributions.
onWsMessage('key_dist', (payload) => {
	receiveSenderKeyDist(
		payload as { channel_id: string; from_user: string; ciphertext: string; ek_public: string }
	).catch((err) => console.error('[signal] Failed to process key_dist:', err));
});

/** Register a handler for a specific message type inside `payload.type`. */
export function onWsMessage(type: string, handler: MessageHandler): () => void {
	if (!handlers.has(type)) handlers.set(type, new Set());
	handlers.get(type)!.add(handler);
	return () => handlers.get(type)?.delete(handler);
}

/** Send a raw JSON frame over the WebSocket (fire-and-forget). */
export function wsSend(msg: Record<string, unknown>): boolean {
	if (socket?.readyState === WebSocket.OPEN) {
		socket.send(JSON.stringify(msg));
		return true;
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

	const ws = new WebSocket(`${WS_URL}/ws`);
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
				break;

			case 'message': {
				const payload = frame.payload as { type?: string } | undefined;
				if (payload?.type) {
					handlers.get(payload.type)?.forEach((h) => h(payload));
				}
				break;
			}

			case 're_auth_required': {
				// Re-send token before it expires
				const token = get(authStore).accessToken;
				if (token) ws.send(JSON.stringify({ type: 'reauth', token }));
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
				break;
		}
	};

	ws.onerror = () => {
		wsStore.update((s) => ({ ...s, error: 'Connection error' }));
	};

	ws.onclose = (event) => {
		clearInterval(pingTimer ?? undefined);
		wsStore.set({ connected: false, error: null });
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
