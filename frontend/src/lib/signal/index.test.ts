import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { IdentityKeyPair, PreKeyPair, SignedPreKey } from './types.js';

vi.mock('$api/client.js', () => ({
	api: {
		get: vi.fn(),
		post: vi.fn(),
	},
}));

vi.mock('./keystore.js', () => ({
	loadIdentityKeyPair: vi.fn(),
	storeIdentityKeyPair: vi.fn(),
	loadLatestSignedPreKey: vi.fn(),
	storeSignedPreKey: vi.fn(),
	storePreKey: vi.fn(),
	listPreKeys: vi.fn(),
	loadSignalBootstrapComplete: vi.fn(),
	storeSignalBootstrapComplete: vi.fn(),
}));

import { api } from '$api/client.js';
import * as ks from './keystore.js';
import { replenishPreKeysIfNeeded, setupKeys } from './index.js';

const identity: IdentityKeyPair = {
	dhPublicKey: new Uint8Array(32).fill(1),
	dhPrivateKey: new Uint8Array(32).fill(2),
	sigPublicKey: new Uint8Array(32).fill(3),
	sigPrivateKey: new Uint8Array(32).fill(4),
};

const signedPreKey: SignedPreKey = {
	keyId: 1,
	publicKey: new Uint8Array(32).fill(5),
	privateKey: new Uint8Array(32).fill(6),
	signature: new Uint8Array(64).fill(7),
	createdAt: Date.now(),
};

const existingPreKeys: PreKeyPair[] = [
	{ keyId: 96, publicKey: new Uint8Array(32).fill(8), privateKey: new Uint8Array(32).fill(9) },
	{ keyId: 97, publicKey: new Uint8Array(32).fill(10), privateKey: new Uint8Array(32).fill(11) },
	{ keyId: 98, publicKey: new Uint8Array(32).fill(12), privateKey: new Uint8Array(32).fill(13) },
	{ keyId: 99, publicKey: new Uint8Array(32).fill(14), privateKey: new Uint8Array(32).fill(15) },
	{ keyId: 100, publicKey: new Uint8Array(32).fill(16), privateKey: new Uint8Array(32).fill(17) },
];

describe('signal bootstrap', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.mocked(api.get).mockReset();
		vi.mocked(api.post).mockReset();
		vi.mocked(ks.storeIdentityKeyPair).mockResolvedValue(undefined);
		vi.mocked(ks.storeSignedPreKey).mockResolvedValue(undefined);
		vi.mocked(ks.storePreKey).mockResolvedValue(undefined);
		vi.mocked(ks.storeSignalBootstrapComplete).mockResolvedValue(undefined);
	});

	it('re-uploads local key material when bootstrap is incomplete', async () => {
		vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(identity);
		vi.mocked(ks.loadSignalBootstrapComplete).mockResolvedValue(false);
		vi.mocked(ks.loadLatestSignedPreKey).mockResolvedValue(signedPreKey);
		vi.mocked(ks.listPreKeys).mockResolvedValue(existingPreKeys);
		vi.mocked(api.post).mockResolvedValue({});

		await setupKeys();

		expect(api.post).toHaveBeenNthCalledWith(1, '/api/v1/keys/identity', expect.any(Object));
		expect(api.post).toHaveBeenNthCalledWith(2, '/api/v1/keys/signed-prekey', expect.any(Object));
		expect(api.post).toHaveBeenNthCalledWith(
			3,
			'/api/v1/keys/one-time-prekeys',
			expect.objectContaining({
				device_id: 1,
				keys: expect.arrayContaining([
					expect.objectContaining({ key_id: 96 }),
					expect.objectContaining({ key_id: 100 }),
				]),
			}),
		);
		expect(ks.storeSignalBootstrapComplete).toHaveBeenCalledWith(true);
	});

	it('replenishes from the max local key id instead of the remaining count', async () => {
		vi.mocked(api.get).mockResolvedValue({ count: 5, low: true });
		vi.mocked(api.post).mockResolvedValue({});
		vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(identity);
		vi.mocked(ks.listPreKeys).mockResolvedValue(existingPreKeys);

		await replenishPreKeysIfNeeded();

		expect(ks.storeSignalBootstrapComplete).toHaveBeenNthCalledWith(1, false);
		expect(api.post).toHaveBeenCalledWith(
			'/api/v1/keys/one-time-prekeys',
			expect.objectContaining({
				device_id: 1,
				keys: expect.arrayContaining([
					expect.objectContaining({ key_id: 101 }),
					expect.objectContaining({ key_id: 200 }),
				]),
			}),
		);
		expect(ks.storeSignalBootstrapComplete).toHaveBeenNthCalledWith(2, true);
	});
});
