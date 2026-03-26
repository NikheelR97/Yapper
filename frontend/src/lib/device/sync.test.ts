import "fake-indexeddb/auto";
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
	apiGet,
	exportSignalSnapshot,
	importSignalSnapshot,
} = vi.hoisted(() => ({
	apiGet: vi.fn(),
	exportSignalSnapshot: vi.fn(),
	importSignalSnapshot: vi.fn(),
}));

vi.mock('$api/client.js', () => ({
	api: {
		get: apiGet,
	},
}));

vi.mock('$signal/keystore.js', () => ({
	exportSignalSnapshot,
	importSignalSnapshot,
}));

import {
	clearPendingDeviceSyncRequest,
	decryptDeviceSyncChunk,
	encryptDeviceSyncChunk,
	ensurePendingDeviceSyncRequest,
	exportTrustedDeviceSnapshot,
	hasPendingDeviceSyncRequest,
	importTrustedDeviceSnapshot,
} from './sync.js';

describe('device sync helpers', () => {
	const deviceId = 'device-sync-test';

	beforeEach(async () => {
		apiGet.mockReset();
		exportSignalSnapshot.mockReset();
		importSignalSnapshot.mockReset();
		await clearPendingDeviceSyncRequest(deviceId);
	});

	it('creates a pending sync request and decrypts encrypted chunks', async () => {
		const request = await ensurePendingDeviceSyncRequest(deviceId);
		expect(await hasPendingDeviceSyncRequest(deviceId)).toBe(true);

		const plaintext = new TextEncoder().encode('trusted history snapshot');
		const encrypted = await encryptDeviceSyncChunk(plaintext, request.publicKey);
		const decrypted = await decryptDeviceSyncChunk(
			deviceId,
			encrypted.ciphertext,
			encrypted.ekPublic,
		);

		expect(new TextDecoder().decode(decrypted)).toBe('trusted history snapshot');
	});

	it('exports merged remote and cached history snapshots', async () => {
		exportSignalSnapshot.mockResolvedValue({
			sessions: [{ sessionId: 'session-1', conversationId: 'conv-1', peerId: 'user-2' }],
			receiverKeys: [{ channelId: 'channel-1', senderId: 'user-2', senderDeviceId: 'device-2' }],
			dmHistory: [
				{
					id: 'dm-local',
					conversation_id: 'conv-1',
					sender_id: 'user-2',
					sender_device_id: 'device-2',
					sender_signal_device_id: 4,
					ciphertext: 'local-cipher',
					ephemeral_key: null,
					opk_id: null,
					msg_num: 2,
					created_at: '2026-03-07T10:00:00Z',
				},
				{
					id: 'dm-shared',
					conversation_id: 'conv-1',
					sender_id: 'user-2',
					sender_device_id: 'device-2',
					sender_signal_device_id: 4,
					ciphertext: 'cached-version',
					ephemeral_key: null,
					opk_id: null,
					msg_num: 3,
					created_at: '2026-03-07T10:05:00Z',
				},
			],
			channelHistory: [
				{
					id: 'channel-local',
					channel_id: 'channel-1',
					sender_id: 'user-2',
					sender_device_id: 'device-2',
					ciphertext: 'local-channel',
					plaintext: null,
					message_type: 'text',
					msg_num: 8,
					created_at: '2026-03-07T11:00:00Z',
				},
				{
					id: 'channel-shared',
					channel_id: 'channel-1',
					sender_id: 'user-2',
					sender_device_id: 'device-2',
					ciphertext: 'cached-channel-version',
					plaintext: null,
					message_type: 'text',
					msg_num: 9,
					created_at: '2026-03-07T11:05:00Z',
				},
			],
		});

		apiGet.mockImplementation(async (path: string) => {
			switch (path) {
				case '/api/v2/conversations':
					return [{ id: 'conv-1' }];
				case '/api/v2/conversations/conv-1/messages?limit=100':
					return [
						{
							id: 'dm-remote',
							conversation_id: 'conv-1',
							sender_id: 'user-3',
							sender_device_id: 'device-3',
							sender_signal_device_id: 5,
							ciphertext: 'remote-cipher',
							ephemeral_key: null,
							opk_id: null,
							msg_num: 1,
							created_at: '2026-03-07T09:00:00Z',
						},
						{
							id: 'dm-shared',
							conversation_id: 'conv-1',
							sender_id: 'user-2',
							sender_device_id: 'device-2',
							sender_signal_device_id: 4,
							ciphertext: 'remote-version',
							ephemeral_key: null,
							opk_id: null,
							msg_num: 3,
							created_at: '2026-03-07T10:05:00Z',
						},
					];
				case '/api/v2/servers':
					return [{ id: 'server-1' }];
				case '/api/v2/servers/server-1/channels':
					return [{ id: 'channel-1' }];
				case '/api/v2/channels/channel-1/messages?limit=100':
					return [
						{
							id: 'channel-remote',
							channel_id: 'channel-1',
							sender_id: 'user-3',
							sender_device_id: 'device-3',
							ciphertext: 'remote-channel',
							plaintext: null,
							message_type: 'text',
							msg_num: 7,
							created_at: '2026-03-07T10:30:00Z',
						},
						{
							id: 'channel-shared',
							channel_id: 'channel-1',
							sender_id: 'user-2',
							sender_device_id: 'device-2',
							ciphertext: 'remote-channel-version',
							plaintext: null,
							message_type: 'text',
							msg_num: 9,
							created_at: '2026-03-07T11:05:00Z',
						},
					];
				default:
					throw new Error(`Unexpected path ${path}`);
			}
		});

		const serialized = await exportTrustedDeviceSnapshot();
		const snapshot = JSON.parse(serialized) as {
			sessions: Array<{ sessionId: string }>;
			receiverKeys: Array<{ channelId: string }>;
			dmHistory: Array<{ id: string; ciphertext: string }>;
			channelHistory: Array<{ id: string; ciphertext: string }>;
		};

		expect(snapshot.sessions).toHaveLength(1);
		expect(snapshot.receiverKeys).toHaveLength(1);
		expect(snapshot.dmHistory.map((message) => message.id)).toEqual([
			'dm-remote',
			'dm-local',
			'dm-shared',
		]);
		expect(snapshot.channelHistory.map((message) => message.id)).toEqual([
			'channel-remote',
			'channel-local',
			'channel-shared',
		]);
		expect(snapshot.dmHistory.find((message) => message.id === 'dm-shared')?.ciphertext).toBe(
			'cached-version',
		);
		expect(
			snapshot.channelHistory.find((message) => message.id === 'channel-shared')?.ciphertext,
		).toBe('cached-channel-version');
	});

	it('pages backward through remote history before exporting', async () => {
		exportSignalSnapshot.mockResolvedValue({
			sessions: [],
			receiverKeys: [],
			dmHistory: [],
			channelHistory: [],
		});

		const latestPage = Array.from({ length: 100 }, (_, index) => {
			const sequence = index + 2;
			const id = `dm-${sequence.toString().padStart(3, '0')}`;
			const minute = Math.floor(index / 60);
			const second = index % 60;
			return {
				id,
				conversation_id: 'conv-1',
				sender_id: 'user-2',
				sender_device_id: 'device-2',
				sender_signal_device_id: 4,
				ciphertext: id,
				ephemeral_key: null,
				opk_id: null,
				msg_num: sequence,
				created_at: `2026-03-07T10:${minute.toString().padStart(2, '0')}:${second
					.toString()
					.padStart(2, '0')}Z`,
			};
		});

		apiGet.mockImplementation(async (path: string) => {
			switch (path) {
				case '/api/v2/conversations':
					return [{ id: 'conv-1' }];
				case '/api/v2/conversations/conv-1/messages?limit=100':
					return latestPage;
				case '/api/v2/servers':
					return [];
				default:
					if (
						path.startsWith(
							'/api/v2/conversations/conv-1/messages?limit=100&before=dm-002',
						)
					) {
						return [
							{
								id: 'dm-oldest',
								conversation_id: 'conv-1',
								sender_id: 'user-2',
								sender_device_id: 'device-2',
								sender_signal_device_id: 4,
								ciphertext: 'oldest',
								ephemeral_key: null,
								opk_id: null,
								msg_num: 1,
								created_at: '2026-03-07T09:00:00Z',
							},
						];
					}
					throw new Error(`Unexpected path ${path}`);
			}
		});

		const serialized = await exportTrustedDeviceSnapshot();
		const snapshot = JSON.parse(serialized) as {
			dmHistory: Array<{ id: string }>;
		};

		expect(snapshot.dmHistory).toHaveLength(101);
		expect(snapshot.dmHistory[0]?.id).toBe('dm-oldest');
		expect(snapshot.dmHistory[1]?.id).toBe('dm-002');
		expect(snapshot.dmHistory.at(-1)?.id).toBe('dm-101');
	});

	it('imports decrypted sync snapshots into the local signal store', async () => {
		await importTrustedDeviceSnapshot(
			JSON.stringify({
				sessions: [{ sessionId: 'session-2' }],
				receiverKeys: [{ channelId: 'channel-2' }],
				dmHistory: [{ id: 'dm-2' }],
				channelHistory: [{ id: 'channel-2' }],
			}),
		);

		expect(importSignalSnapshot).toHaveBeenCalledWith({
			sessions: [{ sessionId: 'session-2' }],
			receiverKeys: [{ channelId: 'channel-2' }],
			dmHistory: [{ id: 'dm-2' }],
			channelHistory: [{ id: 'channel-2' }],
		});
	});
});
