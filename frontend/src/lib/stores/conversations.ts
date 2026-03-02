/**
 * DM Conversations store.
 *
 * Keeps a sorted list of conversations and their decrypted message history.
 * Decryption happens on the fly via the Signal module.
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { decryptDm, encryptDm } from '$signal/index.js';
import { onWsMessage, sendDmMessage } from '$stores/ws.js';
import { authStore } from '$stores/auth.js';

// ─── Types ────────────────────────────────────────────────────────────────────

export interface Conversation {
	id: string;
	peerId: string;
	peerUsername: string;
	peerDisplayName: string | null;
	peerAvatarUrl: string | null;
	lastMessageAt: string | null;
}

export interface Message {
	id: string;
	conversationId: string;
	senderId: string;
	/** Decrypted plaintext. Null while decryption is in progress. */
	text: string | null;
	/** True if decryption failed. */
	decryptError: boolean;
	createdAt: string;
}

interface ConversationStore {
	conversations: Conversation[];
	loading: boolean;
}

// ─── Stores ───────────────────────────────────────────────────────────────────

export const conversationsStore = writable<ConversationStore>({
	conversations: [],
	loading: false,
});

// Per-conversation message lists
const messageStores = new Map<string, ReturnType<typeof writable<Message[]>>>();

export function getMessageStore(conversationId: string) {
	if (!messageStores.has(conversationId)) {
		messageStores.set(conversationId, writable<Message[]>([]));
	}
	return messageStores.get(conversationId)!;
}

// ─── Fetch Conversations ──────────────────────────────────────────────────────

export async function fetchConversations(): Promise<void> {
	conversationsStore.update((s) => ({ ...s, loading: true }));
	try {
		const raw = await api.get<
			{
				id: string;
				peer_id: string;
				peer_username: string;
				peer_display_name: string | null;
				peer_avatar_url: string | null;
				last_message_at: string | null;
			}[]
		>('/api/v1/conversations');

		conversationsStore.set({
			conversations: raw.map((c) => ({
				id: c.id,
				peerId: c.peer_id,
				peerUsername: c.peer_username,
				peerDisplayName: c.peer_display_name,
				peerAvatarUrl: c.peer_avatar_url,
				lastMessageAt: c.last_message_at,
			})),
			loading: false,
		});
	} catch {
		conversationsStore.update((s) => ({ ...s, loading: false }));
	}
}

// ─── Load Message History ─────────────────────────────────────────────────────

export async function loadMessages(conversationId: string, peerId: string): Promise<void> {
	const store = getMessageStore(conversationId);
	const raw = await api.get<
		{
			id: string;
			conversation_id: string;
			sender_id: string;
			ciphertext: string;
			ephemeral_key: string | null;
			opk_id: number | null;
			created_at: string;
		}[]
	>(`/api/v1/conversations/${conversationId}/messages`);

	const messages: Message[] = await Promise.all(
		raw.map(async (m) => {
			try {
				const text = await decryptDm(conversationId, peerId, {
					ciphertext: m.ciphertext,
					ephemeral_key: m.ephemeral_key,
					opk_id: m.opk_id,
				});
				return {
					id: m.id,
					conversationId: m.conversation_id,
					senderId: m.sender_id,
					text,
					decryptError: false,
					createdAt: m.created_at,
				};
			} catch {
				return {
					id: m.id,
					conversationId: m.conversation_id,
					senderId: m.sender_id,
					text: null,
					decryptError: true,
					createdAt: m.created_at,
				};
			}
		})
	);

	store.set(messages);
}

// ─── Send Message ─────────────────────────────────────────────────────────────

export async function sendMessage(conversationId: string, peerId: string, text: string): Promise<void> {
	const encrypted = await encryptDm(conversationId, peerId, text);

	const sent = sendDmMessage(
		conversationId,
		encrypted.ciphertext,
		encrypted.msgNum,
		encrypted.ephemeralKey,
		encrypted.opkId
	);

	if (!sent) {
		throw new Error('WebSocket not connected — message not sent');
	}

	// Optimistically append to local store as own message
	const userId = get(authStore).user?.id ?? '';
	const store = getMessageStore(conversationId);
	store.update((msgs) => [
		...msgs,
		{
			id: crypto.randomUUID(),
			conversationId,
			senderId: userId,
			text,
			decryptError: false,
			createdAt: new Date().toISOString(),
		},
	]);
}

// ─── WS Incoming DM Handler ───────────────────────────────────────────────────

/** Register the global WS handler for incoming DMs. Call once on app init. */
export function registerDmHandler(): () => void {
	return onWsMessage('dm', async (payload) => {
		const msg = payload as {
			id: string;
			conversation_id: string;
			sender_id: string;
			ciphertext: string;
			ephemeral_key?: string | null;
			opk_id?: number | null;
		};

		// Look up the peer id from our conversations store
		const conv = get(conversationsStore).conversations.find((c) => c.id === msg.conversation_id);
		const peerId = conv?.peerId ?? msg.sender_id;

		let text: string | null = null;
		let decryptError = false;
		try {
			text = await decryptDm(msg.conversation_id, peerId, {
				ciphertext: msg.ciphertext,
				ephemeral_key: msg.ephemeral_key ?? null,
				opk_id: msg.opk_id ?? null,
			});
		} catch {
			decryptError = true;
		}

		const store = getMessageStore(msg.conversation_id);
		store.update((msgs) => [
			...msgs,
			{
				id: msg.id,
				conversationId: msg.conversation_id,
				senderId: msg.sender_id,
				text,
				decryptError,
				createdAt: new Date().toISOString(),
			},
		]);

		// Bump lastMessageAt in conversation list
		conversationsStore.update((s) => ({
			...s,
			conversations: s.conversations.map((c) =>
				c.id === msg.conversation_id
					? { ...c, lastMessageAt: new Date().toISOString() }
					: c
			),
		}));
	});
}
