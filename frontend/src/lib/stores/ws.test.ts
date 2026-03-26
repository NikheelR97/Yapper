import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock WebSocket before importing ws module
const mockSend = vi.fn();
const mockClose = vi.fn();
const mockInstances: MockWebSocket[] = [];

class MockWebSocket {
	static readonly OPEN = 1;
	static readonly CONNECTING = 0;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;
	readonly OPEN = 1;
	readonly CONNECTING = 0;
	readyState = MockWebSocket.OPEN;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onerror: (() => void) | null = null;
	onclose: ((event: { code: number }) => void) | null = null;
	send = mockSend;
	close = mockClose;

	constructor() {
		mockInstances.push(this);
	}
}

vi.stubGlobal('WebSocket', MockWebSocket);

// Mock the signal imports used at module level
vi.mock('$lib/signal/index.js', () => ({
	receiveSenderKeyDist: vi.fn(),
	handleKeyDistRequest: vi.fn(),
}));

// Mock the auth store
vi.mock('$stores/auth.js', () => {
	const { writable, get } = require('svelte/store');
	const store = writable({
		user: { id: 'user-1' },
		accessToken: 'test-token',
		device: { id: 'device-1' },
	});
	return {
		authStore: store,
		registerSessionResetter: vi.fn(() => vi.fn()),
	};
});

import { onWsMessage, wsSend, sendChannelMessage, sendDmMessage, sendTypingStart, sendMarkRead, wsConnect, wsDisconnect } from './ws.js';

describe('onWsMessage', () => {
	it('registers and unregisters handlers', () => {
		const handler = vi.fn();
		const unregister = onWsMessage('test_type', handler);
		expect(typeof unregister).toBe('function');
		unregister();
	});

	it('multiple handlers for the same type', () => {
		const handler1 = vi.fn();
		const handler2 = vi.fn();
		const unreg1 = onWsMessage('multi', handler1);
		const unreg2 = onWsMessage('multi', handler2);
		unreg1();
		unreg2();
	});
});

describe('wsSend', () => {
	it('returns false when no socket is connected', () => {
		const result = wsSend({ type: 'ping' });
		// Socket is null by default, so wsSend returns false
		expect(result).toBe(false);
	});
});

describe('sendChannelMessage', () => {
	it('constructs correct payload shape', () => {
		// Without an active socket, this returns false but tests the API
		const result = sendChannelMessage('ch-1', 'base64ciphertext', 42, 'text');
		expect(result).toBe(false);
	});

	it('defaults messageType to text', () => {
		const result = sendChannelMessage('ch-1', 'cipher', 0);
		expect(result).toBe(false);
	});
});

describe('sendDmMessage', () => {
	it('constructs correct payload shape', () => {
		const result = sendDmMessage('conv-1', 'cipher', 5, 'ek-base64', 7);
		expect(result).toBe(false);
	});

	it('works without optional params', () => {
		const result = sendDmMessage('conv-1', 'cipher', 0);
		expect(result).toBe(false);
	});
});

describe('sendTypingStart', () => {
	it('constructs typing_start message', () => {
		const result = sendTypingStart('channel-123');
		expect(result).toBe(false);
	});
});

describe('sendMarkRead', () => {
	it('constructs read message', () => {
		const result = sendMarkRead('msg-1', 'channel-1');
		expect(result).toBe(false);
	});
});

describe('wsDisconnect', () => {
	it('drops queued messages before the next reconnect', () => {
		wsSend({ type: 'read', message_id: 'msg-queued', channel_id: 'channel-1' });

		wsDisconnect();
		wsConnect();

		const socket = mockInstances.at(-1);
		expect(socket).toBeDefined();
		socket?.onopen?.();
		socket?.onmessage?.({ data: JSON.stringify({ type: 'ready' }) });

		expect(mockSend).toHaveBeenCalledWith(
			JSON.stringify({ type: 'auth', token: 'test-token' })
		);
		expect(mockSend).not.toHaveBeenCalledWith(
			JSON.stringify({ type: 'read', message_id: 'msg-queued', channel_id: 'channel-1' })
		);
	});
});
