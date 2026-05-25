import { afterEach, describe, expect, it, vi } from 'vitest';
import { ed25519, x25519 } from '@noble/curves/ed25519.js';
import { x3dhInitiate, x3dhRespond } from './x3dh.js';
import type { IdentityKeyPair, KeyBundle, PreKeyPair, SignedPreKey } from './types.js';

const randomSecretKeyMock = vi.hoisted(() => vi.fn<() => Uint8Array>());

vi.mock('@noble/curves/ed25519.js', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@noble/curves/ed25519.js')>();
	randomSecretKeyMock.mockImplementation(() => actual.x25519.utils.randomSecretKey());

	return {
		...actual,
		x25519: {
			...actual.x25519,
			utils: {
				...actual.x25519.utils,
				randomSecretKey: randomSecretKeyMock,
			},
		},
	};
});

function bytesToB64(bytes: Uint8Array): string {
	let binary = '';
	for (let index = 0; index < bytes.length; index += 1) {
		binary += String.fromCharCode(bytes[index]);
	}
	return btoa(binary);
}

function identity(seed = 1): IdentityKeyPair {
	const dhPrivateKey = new Uint8Array(32).fill(seed);
	const sigPrivateKey = new Uint8Array(32).fill(seed + 1);
	return {
		dhPrivateKey,
		dhPublicKey: x25519.getPublicKey(dhPrivateKey),
		sigPrivateKey,
		sigPublicKey: ed25519.getPublicKey(sigPrivateKey),
	};
}

function signedPreKey(identityKey: IdentityKeyPair, seed = 9): SignedPreKey {
	const privateKey = new Uint8Array(32).fill(seed);
	const publicKey = x25519.getPublicKey(privateKey);
	return {
		keyId: seed,
		publicKey,
		privateKey,
		signature: ed25519.sign(publicKey, identityKey.sigPrivateKey),
		createdAt: Date.now(),
	};
}

function bundleFrom(
	userId: string,
	deviceId: string,
	identityKey: IdentityKeyPair,
	spk: SignedPreKey,
	opk: PreKeyPair | null,
): KeyBundle {
	return {
		userId,
		deviceId,
		signalDeviceId: 1,
		identity_dh_key: bytesToB64(identityKey.dhPublicKey),
		identity_sig_key: bytesToB64(identityKey.sigPublicKey),
		signed_prekey_id: spk.keyId,
		signed_prekey: bytesToB64(spk.publicKey),
		signed_prekey_sig: bytesToB64(spk.signature),
		one_time_prekey_id: opk?.keyId ?? null,
		one_time_prekey: opk ? bytesToB64(opk.publicKey) : null,
	};
}

function preKey(seed: number): PreKeyPair {
	const privateKey = new Uint8Array(32).fill(seed);
	return {
		keyId: seed,
		privateKey,
		publicKey: x25519.getPublicKey(privateKey),
	};
}

function mockEphemeralPrivateKey(seed: number) {
	randomSecretKeyMock.mockReturnValueOnce(new Uint8Array(32).fill(seed));
}

afterEach(() => {
	randomSecretKeyMock.mockReset();
});

describe('x3dh', () => {
	it('derives a 32-byte root key when an OPK is present', async () => {
		const alice = identity(3);
		const bob = identity(11);
		const spk = signedPreKey(bob, 19);
		const opk = preKey(23);
		const bundle = bundleFrom('bob', 'bob-device', bob, spk, opk);

		mockEphemeralPrivateKey(31);
		const result = await x3dhInitiate(alice, bundle, 'session-a', 'conversation-1');

		expect(result.session.rootKey).toHaveLength(32);
		expect(result.session.sendChainKey).toHaveLength(32);
		expect(result.usedOpkId).toBe(opk.keyId);
		expect(result.ephemeralPublicKey).toHaveLength(32);
	});

	it('falls back to the 3-DH path when no OPK is available', async () => {
		const alice = identity(5);
		const bob = identity(13);
		const spk = signedPreKey(bob, 21);
		const withOpk = bundleFrom('bob', 'bob-device', bob, spk, preKey(25));
		const withoutOpk = bundleFrom('bob', 'bob-device', bob, spk, null);

		mockEphemeralPrivateKey(33);
		const opkResult = await x3dhInitiate(alice, withOpk, 'session-opk', 'conversation-1');
		mockEphemeralPrivateKey(33);
		const fallbackResult = await x3dhInitiate(alice, withoutOpk, 'session-fallback', 'conversation-1');

		expect(fallbackResult.session.rootKey).toHaveLength(32);
		expect(fallbackResult.usedOpkId).toBeNull();
		expect(fallbackResult.session.rootKey).not.toEqual(opkResult.session.rootKey);
	});

	it('produces different master secrets for different ephemeral keys', async () => {
		const alice = identity(7);
		const bob = identity(15);
		const spk = signedPreKey(bob, 27);
		const bundle = bundleFrom('bob', 'bob-device', bob, spk, null);

		mockEphemeralPrivateKey(35);
		const first = await x3dhInitiate(alice, bundle, 'session-1', 'conversation-1');
		mockEphemeralPrivateKey(36);
		const second = await x3dhInitiate(alice, bundle, 'session-2', 'conversation-1');

		expect(first.session.rootKey).not.toEqual(second.session.rootKey);
	});

	it('round-trips the same root key on the responder side', async () => {
		const alice = identity(9);
		const bob = identity(17);
		const spk = signedPreKey(bob, 29);
		const opk = preKey(31);
		const bundle = bundleFrom('bob', 'bob-device', bob, spk, opk);

		mockEphemeralPrivateKey(39);
		const initiated = await x3dhInitiate(alice, bundle, 'session-init', 'conversation-1');
		const responded = await x3dhRespond(
			bob,
			spk,
			opk,
			initiated.ephemeralPublicKey,
			alice.dhPublicKey,
			'session-respond',
			'conversation-1',
			'alice',
			'alice-device',
			1,
		);

		expect(initiated.session.rootKey).toEqual(responded.rootKey);
		expect(initiated.session.sendChainKey).toHaveLength(32);
		expect(responded.receiveChainKey).toHaveLength(32);
	});

	it('rejects a malformed bundle before session creation', async () => {
		const alice = identity(11);
		const malformed = {
			userId: 'bob',
			deviceId: 'bob-device',
			signalDeviceId: 1,
			identity_dh_key: bytesToB64(alice.dhPublicKey),
			identity_sig_key: bytesToB64(alice.sigPublicKey),
			signed_prekey_id: 1,
			signed_prekey: '',
			signed_prekey_sig: '',
			one_time_prekey_id: null,
			one_time_prekey: null,
		} satisfies KeyBundle;

		await expect(x3dhInitiate(alice, malformed, 'bad-session', 'conversation-1')).rejects.toThrow(
			/signed prekey/i,
		);
	});
});
