/**
 * DM Conversations store.
 *
 * Keeps a sorted list of conversations and their decrypted message history.
 * Decryption happens on the fly via the Signal module.
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { decryptDm, encryptDm } from '$signal/index.js';
import {
	listDmHistoryMessages,
	storeDmHistoryMessages,
	type CachedDmHistoryMessage,
} from '$signal/keystore.js';
import { onWsMessage } from '$stores/ws.js';
import { authStore } from '$stores/auth.js';

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
	/** 'text' | 'yap' | 'clip' */
	messageType: string;
}

interface ConversationStore {
	conversations: Conversation[];
	loading: boolean;
	loadError: boolean;
}

interface RawConversation {
	id: string;
	peer_id: string;
	peer_username: string;
	peer_display_name: string | null;
	peer_avatar_url: string | null;
	last_message_at: string | null;
}

interface RawMessageV2 {
	id: string;
	conversation_id: string;
	sender_id: string;
	sender_device_id: string;
	sender_signal_device_id: number;
	ciphertext: string;
	ephemeral_key: string | null;
	opk_id: number | null;
	msg_num: number;
	ratchet_pub: string | null;
	previous_chain_len: number | null;
	crypto_version: number | null;
	created_at: string;
}

function mergeDmHistoryMessages(
	remoteMessages: RawMessageV2[],
	cachedMessages: CachedDmHistoryMessage[]
): RawMessageV2[] {
	const messagesById = new Map<string, RawMessageV2>();
	for (const message of cachedMessages) {
		messagesById.set(message.id, message);
	}
	for (const message of remoteMessages) {
		messagesById.set(message.id, message);
	}
	return [...messagesById.values()].sort(
		(a, b) => Date.parse(a.created_at) - Date.parse(b.created_at)
	);
}

export const conversationsStore = writable<ConversationStore>({
	conversations: [],
	loading: false,
	loadError: false,
});

const messageStores = new Map<string, ReturnType<typeof writable<Message[]>>>();

export function getMessageStore(conversationId: string) {
	if (!messageStores.has(conversationId)) {
		messageStores.set(conversationId, writable<Message[]>([]));
	}
	return messageStores.get(conversationId)!;
}

function inferDmMessageType(text: string | null): string {
	if (!text) return 'text';
	try {
		const payload = JSON.parse(text) as { mime_type?: string };
		if (typeof payload.mime_type !== 'string') return 'text';
		if (payload.mime_type.startsWith('audio/')) return 'yap';
		if (payload.mime_type.startsWith('video/')) return 'clip';
	} catch {
		// Not a structured media payload.
	}
	return 'text';
}

function touchConversation(conversationId: string, timestamp: string): void {
	conversationsStore.update((state) => ({
		...state,
		conversations: [...state.conversations]
			.map((conversation) =>
				conversation.id === conversationId
					? { ...conversation, lastMessageAt: timestamp }
					: conversation
			)
			.sort((a, b) => {
				const aTime = a.lastMessageAt ? Date.parse(a.lastMessageAt) : 0;
				const bTime = b.lastMessageAt ? Date.parse(b.lastMessageAt) : 0;
				return bTime - aTime;
			}),
	}));
}

export async function fetchConversations(): Promise<void> {
	conversationsStore.update((state) => ({ ...state, loading: true }));
	try {
		const raw = await api.get<RawConversation[]>('/api/v2/conversations');
		conversationsStore.set({
			conversations: raw.map((conversation) => ({
				id: conversation.id,
				peerId: conversation.peer_id,
				peerUsername: conversation.peer_username,
				peerDisplayName: conversation.peer_display_name,
				peerAvatarUrl: conversation.peer_avatar_url,
				lastMessageAt: conversation.last_message_at,
			})),
			loading: false,
			loadError: false,
		});
	} catch {
		conversationsStore.update((state) => ({ ...state, loading: false, loadError: true }));
	}
}

export async function loadMessages(conversationId: string, peerId: string): Promise<void> {
	const store = getMessageStore(conversationId);
	const [remoteMessages, cachedMessages] = await Promise.all([
		api.get<RawMessageV2[]>(`/api/v2/conversations/${conversationId}/messages`),
		listDmHistoryMessages(conversationId),
	]);
	const raw = mergeDmHistoryMessages(remoteMessages, cachedMessages);
	await storeDmHistoryMessages(remoteMessages);

	const messages: Message[] = await Promise.all(
		raw.map(async (message) => {
			try {
				const text = await decryptDm(
					conversationId,
					peerId,
					message.sender_device_id,
					message.sender_signal_device_id,
					{
						ciphertext: message.ciphertext,
						ephemeral_key: message.ephemeral_key,
						opk_id: message.opk_id,
						msg_num: message.msg_num,
						ratchet_pub: message.ratchet_pub,
						previous_chain_len: message.previous_chain_len,
						crypto_version: message.crypto_version,
					}
				);
				return {
					id: message.id,
					conversationId: message.conversation_id,
					senderId: message.sender_id,
					text,
					decryptError: false,
					createdAt: message.created_at,
					messageType: inferDmMessageType(text),
				};
			} catch {
				return {
					id: message.id,
					conversationId: message.conversation_id,
					senderId: message.sender_id,
					text: null,
					decryptError: true,
					createdAt: message.created_at,
					messageType: 'text',
				};
			}
		})
	);

	store.set(messages);
}

export async function sendMessage(conversationId: string, peerId: string, text: string): Promise<void> {
	const encrypted = await encryptDm(conversationId, peerId, text);
	const response = await api.post<{ status: string; message_id: string; created_at: string }>(
		`/api/v2/conversations/${conversationId}/messages`,
		{
			envelopes: encrypted.envelopes.map((envelope) => ({
				recipient_user_id: envelope.recipientUserId,
				recipient_device_id: envelope.recipientDeviceId,
				ciphertext: envelope.ciphertext,
				ephemeral_key: envelope.ephemeralKey ?? null,
				opk_id: envelope.opkId ?? null,
				msg_num: envelope.msgNum,
				ratchet_pub: envelope.ratchetPub ?? null,
				previous_chain_len: envelope.previousChainLen ?? null,
				crypto_version: envelope.cryptoVersion,
			})),
		}
	);

	const userId = get(authStore).user?.id ?? '';
	const createdAt = response.created_at ?? new Date().toISOString();
	const store = getMessageStore(conversationId);
	store.update((messages) => [
		...messages,
		{
			id: response.message_id,
			conversationId,
			senderId: userId,
			text,
			decryptError: false,
			createdAt,
			messageType: inferDmMessageType(text),
		},
	]);
	touchConversation(conversationId, createdAt);
}

export function registerDmHandler(): () => void {
	return onWsMessage('dm_v2', async (payload) => {
		const message = payload as RawMessageV2;
		await storeDmHistoryMessages([message]);
		const conversation = get(conversationsStore).conversations.find(
			(entry) => entry.id === message.conversation_id
		);
		const currentUserId = get(authStore).user?.id;
		const peerId = conversation?.peerId ?? (message.sender_id === currentUserId ? currentUserId ?? '' : message.sender_id);

		let text: string | null = null;
		let decryptError = false;
		try {
			text = await decryptDm(
				message.conversation_id,
				peerId,
				message.sender_device_id,
				message.sender_signal_device_id,
				{
					ciphertext: message.ciphertext,
					ephemeral_key: message.ephemeral_key ?? null,
					opk_id: message.opk_id ?? null,
					msg_num: message.msg_num,
					ratchet_pub: message.ratchet_pub ?? null,
					previous_chain_len: message.previous_chain_len ?? null,
					crypto_version: message.crypto_version,
				}
			);
		} catch {
			decryptError = true;
		}

		const createdAt = message.created_at ?? new Date().toISOString();
		const store = getMessageStore(message.conversation_id);
		store.update((messages) => [
			...messages,
			{
				id: message.id,
				conversationId: message.conversation_id,
				senderId: message.sender_id,
				text,
				decryptError,
				createdAt,
				messageType: inferDmMessageType(text),
			},
		]);
		touchConversation(message.conversation_id, createdAt);
	});
}
