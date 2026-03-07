import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api/client.js', () => ({
	api: {
		get: vi.fn(),
		post: vi.fn(),
		put: vi.fn(),
	},
}));

vi.mock('./keystore.js', () => ({
	exportSignalSnapshot: vi.fn(),
	importSignalSnapshot: vi.fn(),
}));

import { api } from '$lib/api/client.js';
import { importSignalSnapshot } from './keystore.js';
import { restoreKeys } from './backup.js';

const PBKDF2_ITERS = 600_000;

function u8ToB64(u8: Uint8Array): string {
	return btoa(String.fromCharCode(...u8));
}

async function deriveKey(pin: string, salt: Uint8Array): Promise<CryptoKey> {
	const keyMaterial = await crypto.subtle.importKey(
		'raw',
		new TextEncoder().encode(pin),
		'PBKDF2',
		false,
		['deriveKey']
	);
	return crypto.subtle.deriveKey(
		{ name: 'PBKDF2', salt: salt.slice(), hash: 'SHA-256', iterations: PBKDF2_ITERS },
		keyMaterial,
		{ name: 'AES-GCM', length: 256 },
		false,
		['encrypt', 'decrypt']
	);
}

async function encryptBackupBlob(pin: string, snapshot: string): Promise<string> {
	const salt = new Uint8Array(16).fill(7);
	const iv = new Uint8Array(12).fill(9);
	const key = await deriveKey(pin, salt);
	const ciphertext = new Uint8Array(
		await crypto.subtle.encrypt(
			{ name: 'AES-GCM', iv },
			key,
			new TextEncoder().encode(snapshot)
		)
	);

	const blob = new Uint8Array(salt.length + iv.length + ciphertext.length);
	blob.set(salt, 0);
	blob.set(iv, salt.length);
	blob.set(ciphertext, salt.length + iv.length);
	return u8ToB64(blob);
}

describe('signal backup restore', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('restores a replacement-device backup and finalizes the handoff', async () => {
		const snapshot = JSON.stringify({ bootstrapComplete: true });
		vi.mocked(api.get).mockResolvedValue({
			encrypted_blob: await encryptBackupBlob('2468', snapshot),
		});
		vi.mocked(api.post).mockResolvedValue({});

		const restored = await restoreKeys('2468', 'source-device-1');

		expect(restored).toBe(true);
		expect(api.get).toHaveBeenCalledWith(
			'/api/v2/keys/backup?source_device_id=source-device-1'
		);
		expect(importSignalSnapshot).toHaveBeenCalledWith({ bootstrapComplete: true });
		expect(api.post).toHaveBeenCalledWith('/api/v2/keys/backup/restore', {
			source_device_id: 'source-device-1',
		});
	});

	it('returns false when no backup exists for the selected source device', async () => {
		vi.mocked(api.get).mockRejectedValue({ status: 404 });

		const restored = await restoreKeys('2468', 'missing-device');

		expect(restored).toBe(false);
		expect(api.post).not.toHaveBeenCalled();
		expect(importSignalSnapshot).not.toHaveBeenCalled();
	});
});
