import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

type WsHandler = (payload: unknown) => void | Promise<void>;
type SenderKeyReadyListener = (event: {
	channelId: string;
	senderUserId: string;
	senderDeviceId: string;
}) => void | Promise<void>;

const wsHandlers = new Map<string, Set<WsHandler>>();
const senderKeyReadyListeners = new Set<SenderKeyReadyListener>();

vi.mock('$api/client.js', () => ({
	api: {
		get: vi.fn(),
		post: vi.fn(),
	},
}));

vi.mock('$signal/index.js', () => ({
	decryptChannel: vi.fn(),
	encryptChannel: vi.fn(),
	fetchPendingKeyDists: vi.fn().mockResolvedValue(undefined),
	onSenderKeyReady: vi.fn((listener: SenderKeyReadyListener) => {
		senderKeyReadyListeners.add(listener);
		return () => senderKeyReadyListeners.delete(listener);
	}),
	prepareChannel: vi.fn(),
}));

vi.mock('$stores/ws.js', () => ({
	onWsMessage: vi.fn((type: string, handler: WsHandler) => {
		if (!wsHandlers.has(type)) {
			wsHandlers.set(type, new Set());
		}
		wsHandlers.get(type)!.add(handler);
		return () => wsHandlers.get(type)?.delete(handler);
	}),
	sendChannelMessage: vi.fn(),
}));

vi.mock('$signal/keystore.js', () => ({
	getCachedEmojis: vi.fn(),
	listChannelHistoryMessages: vi.fn(),
	setCachedEmojis: vi.fn(),
	storeChannelHistoryMessages: vi.fn().mockResolvedValue(undefined),
}));

import { authStore } from '$stores/auth.js';
import { clearAuth } from '$stores/auth.js';
import { api } from '$api/client.js';
import {
	decryptChannel,
	encryptChannel,
	fetchPendingKeyDists,
} from '$signal/index.js';
import {
	listChannelHistoryMessages,
	storeChannelHistoryMessages,
} from '$signal/keystore.js';
import { sendChannelMessage } from '$stores/ws.js';
import { getChannelMessageStore, registerChannelHandler, sendMessage, serversStore } from './servers.js';
import { conversationsStore, getMessageStore } from './conversations.js';

async function emitWs(type: string, payload: unknown): Promise<void> {
	const handlers = [...(wsHandlers.get(type) ?? [])];
	await Promise.all(handlers.map((handler) => handler(payload)));
}

async function emitSenderKeyReady(event: {
	channelId: string;
	senderUserId: string;
	senderDeviceId: string;
}): Promise<void> {
	await Promise.all(
		[...senderKeyReadyListeners].map((listener) => listener(event))
	);
}

describe('registerChannelHandler', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
		wsHandlers.clear();
		senderKeyReadyListeners.clear();
		authStore.set({
			user: {
				id: 'viewer-user',
				username: 'viewer',
				displayName: 'Viewer',
				avatarUrl: null,
				accountType: 'standard',
				isPremium: false,
			},
			device: {
				id: 'viewer-device',
				signalDeviceId: 1,
				installationId: 'viewer-installation',
				platform: 'web',
				label: 'Viewer Browser',
				trustState: 'trusted',
				createdAt: new Date().toISOString(),
				lastSeenAt: null,
				approvedAt: new Date().toISOString(),
				revokedAt: null,
			},
			accessToken: 'token',
			csrfToken: 'csrf',
			loading: false,
		});
		getChannelMessageStore('channel-1').set([]);
		vi.mocked(api.post).mockResolvedValue({});
		vi.mocked(encryptChannel).mockResolvedValue({
			wireCiphertext: 'ciphertext',
			iteration: 7,
		});
		vi.mocked(sendChannelMessage).mockReturnValue(true);
	});

	afterEach(() => {
		vi.runOnlyPendingTimers();
		vi.useRealTimers();
	});

	it('retries a live channel message after the sender key arrives', async () => {
		vi.mocked(decryptChannel)
			.mockRejectedValueOnce(new Error('No SenderKey'))
			.mockResolvedValueOnce('hello from sender');
		vi.mocked(listChannelHistoryMessages).mockResolvedValue([
			{
				id: 'message-1',
				channel_id: 'channel-1',
				sender_id: 'sender-user',
				sender_device_id: 'sender-device',
				ciphertext: 'ciphertext',
				plaintext: null,
				message_type: 'text',
				msg_num: 7,
				created_at: '2026-03-16T10:00:00.000Z',
			},
		]);

		const unregister = registerChannelHandler();

		try {
			await emitWs('channel', {
				id: 'message-1',
				channel_id: 'channel-1',
				sender_id: 'sender-user',
				sender_device_id: 'sender-device',
				ciphertext: 'ciphertext',
				message_type: 'text',
				msg_num: 7,
				created_at: '2026-03-16T10:00:00.000Z',
			});

			expect(storeChannelHistoryMessages).toHaveBeenCalledOnce();
			expect(fetchPendingKeyDists).toHaveBeenCalledWith('channel-1');

			let messages = get(getChannelMessageStore('channel-1'));
			expect(messages).toHaveLength(1);
			expect(messages[0]?.text).toBeNull();
			expect(messages[0]?.decryptError).toBe(false);

			await emitSenderKeyReady({
				channelId: 'channel-1',
				senderUserId: 'sender-user',
				senderDeviceId: 'sender-device',
			});

			messages = get(getChannelMessageStore('channel-1'));
			expect(messages[0]?.text).toBe('hello from sender');
			expect(messages[0]?.decryptError).toBe(false);
		} finally {
			unregister();
		}
	});

	it('marks the message as a decrypt error if no sender key arrives in time', async () => {
		vi.mocked(decryptChannel).mockRejectedValue(new Error('No SenderKey'));

		const unregister = registerChannelHandler();

		try {
			await emitWs('channel', {
				id: 'message-2',
				channel_id: 'channel-1',
				sender_id: 'sender-user',
				sender_device_id: 'sender-device',
				ciphertext: 'ciphertext',
				message_type: 'text',
				msg_num: 8,
				created_at: '2026-03-16T10:05:00.000Z',
			});

			let messages = get(getChannelMessageStore('channel-1'));
			expect(messages[0]?.text).toBeNull();
			expect(messages[0]?.decryptError).toBe(false);

			await vi.advanceTimersByTimeAsync(2_500);

			messages = get(getChannelMessageStore('channel-1'));
			expect(messages[0]?.text).toBeNull();
			expect(messages[0]?.decryptError).toBe(true);
		} finally {
			unregister();
		}
	});

	it('falls back to the HTTP channel send endpoint when websocket delivery is unavailable', async () => {
		vi.mocked(sendChannelMessage).mockReturnValue(false);
		vi.mocked(api.post).mockResolvedValue({
			id: 'stored-message-1',
			created_at: '2026-03-18T01:00:00.000Z',
			message_type: 'text',
		});

		const unregister = registerChannelHandler();

		try {
			await sendMessage('channel-1', 'hello over http');

			expect(api.post).toHaveBeenCalledWith('/api/v2/channels/channel-1/messages', {
				ciphertext: 'ciphertext',
				message_type: 'text',
				msg_num: 7,
			});

			const messages = get(getChannelMessageStore('channel-1'));
			expect(messages).toHaveLength(1);
			expect(messages[0]).toMatchObject({
				id: 'stored-message-1',
				conversationId: 'channel-1',
				senderId: 'viewer-user',
				senderDeviceId: 'viewer-device',
				text: 'hello over http',
				decryptError: false,
				createdAt: '2026-03-18T01:00:00.000Z',
				messageType: 'text',
				pendingMsgNum: 7,
				deliveryState: 'sent',
			});

			await emitWs('channel', {
				id: 'server-message-2',
				channel_id: 'channel-1',
				sender_id: 'viewer-user',
				sender_device_id: 'viewer-device',
				ciphertext: 'ciphertext',
				message_type: 'text',
				msg_num: 7,
				created_at: '2026-03-18T01:00:01.000Z',
			});

			expect(get(getChannelMessageStore('channel-1'))).toEqual([
				{
					id: 'server-message-2',
					conversationId: 'channel-1',
					senderId: 'viewer-user',
					senderDeviceId: 'viewer-device',
					text: 'hello over http',
					decryptError: false,
					createdAt: '2026-03-18T01:00:01.000Z',
					messageType: 'text',
					pendingMsgNum: null,
					deliveryState: 'sent',
				},
			]);
		} finally {
			unregister();
		}
	});

	it('reconciles an optimistic channel send to the echoed server id', async () => {
		vi.mocked(sendChannelMessage).mockReturnValue(true);

		const unregister = registerChannelHandler();

		try {
			await sendMessage('channel-1', '{"mime_type":"video/mp4"}', { messageType: 'clip' });

			let messages = get(getChannelMessageStore('channel-1'));
			expect(messages).toHaveLength(1);
			expect(messages[0]).toMatchObject({
				conversationId: 'channel-1',
				senderId: 'viewer-user',
				senderDeviceId: 'viewer-device',
				text: '{"mime_type":"video/mp4"}',
				decryptError: false,
				messageType: 'clip',
				pendingMsgNum: 7,
				deliveryState: 'pending',
			});
			const localId = messages[0]?.id;

			await emitWs('channel', {
				id: 'server-message-1',
				channel_id: 'channel-1',
				sender_id: 'viewer-user',
				sender_device_id: 'viewer-device',
				ciphertext: 'ciphertext',
				message_type: 'clip',
				msg_num: 7,
				created_at: '2026-03-18T02:00:00.000Z',
			});

			messages = get(getChannelMessageStore('channel-1'));
			expect(messages).toHaveLength(1);
			expect(messages[0]).toMatchObject({
				id: 'server-message-1',
				conversationId: 'channel-1',
				senderId: 'viewer-user',
				senderDeviceId: 'viewer-device',
				text: '{"mime_type":"video/mp4"}',
				decryptError: false,
				createdAt: '2026-03-18T02:00:00.000Z',
				messageType: 'clip',
				pendingMsgNum: null,
				deliveryState: 'sent',
			});
			expect(messages[0]?.id).not.toBe(localId);
		} finally {
			unregister();
		}
	});

	it('clears session-scoped channel and DM caches when auth is cleared', async () => {
		serversStore.set({
			servers: [
				{
					id: 'server-1',
					name: 'Server 1',
					slug: 'server-1',
					iconUrl: null,
					isOwner: false,
					memberCount: 3,
				},
			],
			loading: false,
		});
		conversationsStore.set({
			conversations: [
				{
					id: 'conv-1',
					peerId: 'peer-1',
					peerUsername: 'peer',
					peerDisplayName: 'Peer',
					peerAvatarUrl: null,
					lastMessageAt: null,
				},
			],
			loading: false,
			loadError: false,
		});
		getMessageStore('conv-1').set([
			{
				id: 'dm-msg-1',
				conversationId: 'conv-1',
				senderId: 'peer-1',
				text: 'hello',
				decryptError: false,
				createdAt: '2026-03-18T03:00:00.000Z',
				messageType: 'text',
			},
		]);
		getChannelMessageStore('channel-1').set([
			{
				id: 'channel-msg-1',
				conversationId: 'channel-1',
				senderId: 'viewer-user',
				senderDeviceId: 'viewer-device',
				text: 'hello',
				decryptError: false,
				createdAt: '2026-03-18T03:05:00.000Z',
				messageType: 'text',
				pendingMsgNum: 1,
				deliveryState: 'pending',
			},
		]);

		clearAuth();

		expect(get(serversStore).servers).toEqual([]);
		expect(get(conversationsStore).conversations).toEqual([]);
		expect(get(getMessageStore('conv-1'))).toEqual([]);
		expect(get(getChannelMessageStore('channel-1'))).toEqual([]);
	});
});
