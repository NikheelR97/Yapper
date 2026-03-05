/**
 * Servers & Channels store.
 *
 * Manages the list of joined servers, per-server channel lists, and per-channel
 * message history. Mirrors the shape of conversations.ts for the group layer.
 */

import { writable, get } from 'svelte/store';
import { api } from '$api/client.js';
import { decryptChannel, encryptChannel, prepareChannel } from '$signal/index.js';
import { onWsMessage, sendChannelMessage as wsSendChannel } from '$stores/ws.js';
import { authStore } from '$stores/auth.js';
import type { Message } from '$stores/conversations.js';
import { getCachedEmojis, setCachedEmojis } from '$signal/keystore.js';

export type { Message };

// ─── Types ────────────────────────────────────────────────────────────────────

export interface ServerEmoji {
	id: string;
	name: string;
	imageUrl: string;
}

export interface Server {
	id: string;
	name: string;
	slug: string;
	iconUrl: string | null;
	isOwner: boolean;
	memberCount: number;
	customEmojis?: ServerEmoji[];
}

export interface Channel {
	id: string;
	serverId: string;
	name: string;
	channelType: string;
	position: number;
}

interface ServersState {
	servers: Server[];
	loading: boolean;
}

// ─── Stores ───────────────────────────────────────────────────────────────────

export const serversStore = writable<ServersState>({ servers: [], loading: false });

// Per-channel message lists
const channelMessageStores = new Map<string, ReturnType<typeof writable<Message[]>>>();

export function getChannelMessageStore(channelId: string) {
	if (!channelMessageStores.has(channelId)) {
		channelMessageStores.set(channelId, writable<Message[]>([]));
	}
	return channelMessageStores.get(channelId)!;
}

// Per-server channel list cache (invalidated by createChannel)
const channelsCache = new Map<string, Channel[]>();

export function invalidateChannels(serverId: string): void {
	channelsCache.delete(serverId);
}

// ─── Fetch Servers ────────────────────────────────────────────────────────────

export async function fetchServers(): Promise<void> {
	serversStore.update((s) => ({ ...s, loading: true }));
	try {
		const raw = await api.get<
			{
				id: string;
				name: string;
				slug: string;
				owner_id: string;
				icon_url: string | null;
				member_count: number;
			}[]
		>('/api/v1/servers');

		const userId = get(authStore).user?.id ?? '';
		serversStore.set({
			servers: raw.map((s) => ({
				id: s.id,
				name: s.name,
				slug: s.slug,
				iconUrl: s.icon_url,
				isOwner: s.owner_id === userId,
				memberCount: s.member_count,
			})),
			loading: false,
		});
	} catch {
		serversStore.update((s) => ({ ...s, loading: false }));
	}
}

// ─── Fetch Channels ───────────────────────────────────────────────────────────

export async function fetchChannels(serverId: string): Promise<Channel[]> {
	const cached = channelsCache.get(serverId);
	if (cached) return cached;

	const raw = await api.get<
		{
			id: string;
			server_id: string;
			name: string;
			type: string;
			position: number;
		}[]
	>(`/api/v1/servers/${serverId}/channels`);

	const channels = raw.map((c) => ({
		id: c.id,
		serverId: c.server_id,
		name: c.name,
		channelType: c.type,
		position: c.position,
	}));
	channelsCache.set(serverId, channels);
	return channels;
}

// ─── Load Channel Messages ────────────────────────────────────────────────────

export async function loadChannelMessages(channelId: string): Promise<void> {
	const store = getChannelMessageStore(channelId);

	const raw = await api.get<
		{
			id: string;
			channel_id: string;
			sender_id: string;
			ciphertext: string | null;
			plaintext: string | null;
			message_type: string;
			msg_num: number | null;
			created_at: string;
		}[]
	>(`/api/v1/channels/${channelId}/messages`);

	const messages: Message[] = await Promise.all(
		raw.map(async (m) => {
			// Bot messages use plaintext column; all user messages use ciphertext
			if (m.plaintext) {
				return {
					id: m.id,
					conversationId: m.channel_id,
					senderId: m.sender_id,
					text: m.plaintext,
					decryptError: false,
					createdAt: m.created_at,
					messageType: m.message_type ?? 'text',
				};
			}
			if (!m.ciphertext) {
				return {
					id: m.id,
					conversationId: m.channel_id,
					senderId: m.sender_id,
					text: null,
					decryptError: true,
					createdAt: m.created_at,
					messageType: 'text',
				};
			}
			try {
				const text = await decryptChannel(m.channel_id, m.sender_id, {
					ciphertext: m.ciphertext,
					msg_num: m.msg_num,
				});
				return {
					id: m.id,
					conversationId: m.channel_id,
					senderId: m.sender_id,
					text,
					decryptError: false,
					createdAt: m.created_at,
					messageType: m.message_type ?? 'text',
				};
			} catch {
				return {
					id: m.id,
					conversationId: m.channel_id,
					senderId: m.sender_id,
					text: null,
					decryptError: true,
					createdAt: m.created_at,
					messageType: 'text',
				};
			}
		})
	);

	store.set(messages);
}

// ─── Send Channel Message ─────────────────────────────────────────────────────

export async function sendMessage(channelId: string, text: string): Promise<void> {
	const { wireCiphertext, iteration } = await encryptChannel(channelId, text);
	const sent = wsSendChannel(channelId, wireCiphertext, iteration);

	if (!sent) {
		throw new Error('WebSocket not connected — message not sent');
	}

	// Optimistically append own message
	const userId = get(authStore).user?.id ?? '';
	const store = getChannelMessageStore(channelId);
	store.update((msgs) => [
		...msgs,
		{
			id: crypto.randomUUID(),
			conversationId: channelId,
			senderId: userId,
			text,
			decryptError: false,
			createdAt: new Date().toISOString(),
			messageType: 'text',
		},
	]);
}

// ─── WS Incoming Channel Message Handler ─────────────────────────────────────

/** Register the global WS handler for incoming channel messages. Call once on app init. */
export function registerChannelHandler(): () => void {
	return onWsMessage('channel', async (payload) => {
		const msg = payload as {
			id: string;
			channel_id: string;
			sender_id: string;
			ciphertext: string;
			msg_num: number | null;
		};

		// Own messages are already added optimistically on send
		const myId = get(authStore).user?.id ?? '';
		if (msg.sender_id === myId) return;

		let text: string | null = null;
		let decryptError = false;
		try {
			text = await decryptChannel(msg.channel_id, msg.sender_id, {
				ciphertext: msg.ciphertext,
				msg_num: msg.msg_num,
			});
		} catch {
			decryptError = true;
		}

		const store = getChannelMessageStore(msg.channel_id);
		store.update((msgs) => [
			...msgs,
			{
				id: msg.id,
				conversationId: msg.channel_id,
				senderId: msg.sender_id,
				text,
				decryptError,
				createdAt: new Date().toISOString(),
				messageType: 'text',
			},
		]);
	});
}

// ─── Create Server ────────────────────────────────────────────────────────────

export async function createServer(name: string): Promise<Server> {
	const raw = await api.post<{
		id: string;
		name: string;
		slug: string;
		owner_id: string;
		icon_url: string | null;
		member_count: number;
	}>('/api/v1/servers', { name });

	const userId = get(authStore).user?.id ?? '';
	const server: Server = {
		id: raw.id,
		name: raw.name,
		slug: raw.slug,
		iconUrl: raw.icon_url,
		isOwner: raw.owner_id === userId,
		memberCount: raw.member_count,
	};

	serversStore.update((s) => ({ ...s, servers: [...s.servers, server] }));
	return server;
}

// ─── Create Invite ────────────────────────────────────────────────────────────

export async function createInvite(serverId: string): Promise<string> {
	const resp = await api.post<{ code: string }>(
		`/api/v1/servers/${serverId}/invite`,
		{}
	);
	return resp.code;
}

// ─── Join by Invite ───────────────────────────────────────────────────────────

export async function joinByInvite(code: string): Promise<void> {
	await api.post(`/api/v1/servers/join/${code}`, {});
	await fetchServers();
}

// ─── Server Emojis ────────────────────────────────────────────────────────────

function applyEmojisToStore(serverId: string, emojis: ServerEmoji[]): void {
	serversStore.update((s) => ({
		...s,
		servers: s.servers.map((srv) =>
			srv.id === serverId ? { ...srv, customEmojis: emojis } : srv,
		),
	}));
}

/** Load custom emojis for a server. Serves from IndexedDB cache immediately,
 *  then always refreshes from the API and persists the result. */
export async function fetchServerEmojis(serverId: string): Promise<void> {
	// Serve cached emojis instantly (no loading flicker)
	try {
		const cached = await getCachedEmojis(serverId);
		if (cached) applyEmojisToStore(serverId, cached);
	} catch {
		// Non-fatal — proceed to API fetch
	}

	// Always refresh from API to pick up new emojis
	try {
		const res = await api.get<{
			emojis: { id: string; name: string; image_url: string }[];
		}>(`/api/v1/servers/${serverId}/emojis`);

		const emojis: ServerEmoji[] = (res.emojis ?? []).map((e) => ({
			id: e.id,
			name: e.name,
			imageUrl: e.image_url,
		}));

		applyEmojisToStore(serverId, emojis);
		setCachedEmojis(serverId, emojis).catch(() => {});
	} catch {
		// Non-fatal — cached or empty emoji list shown
	}
}

/** Register a WS handler that refreshes the emoji cache when a new emoji is added. */
export function registerEmojiHandler(): () => void {
	return onWsMessage('emoji_added', (payload) => {
		const p = payload as { emoji?: { server_id?: string } };
		if (p.emoji?.server_id) fetchServerEmojis(p.emoji.server_id).catch(() => {});
	});
}
