/**
 * X3DH (Extended Triple Diffie-Hellman) key agreement.
 *
 * Protocol: https://signal.org/docs/specifications/x3dh/
 *
 * Key types used:
 *   IK  — Identity Key       (X25519 for DH)
 *   SPK — Signed PreKey      (X25519, signed with Ed25519)
 *   OPK — One-Time PreKey    (X25519, unsigned)
 *   EK  — Ephemeral Key      (X25519, generated per session init)
 *
 * Initiator (Alice) computes:
 *   DH1 = DH(IK_A, SPK_B)
 *   DH2 = DH(EK_A, IK_B)
 *   DH3 = DH(EK_A, SPK_B)
 *   DH4 = DH(EK_A, OPK_B)  [if OPK available]
 *   SK  = KDF(F || DH1 || DH2 || DH3 [|| DH4])
 *
 * where F = 0xFF × 32 (domain separator from Signal X3DH spec §2.3)
 */

import { ed25519, x25519 } from '@noble/curves/ed25519.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { sha256 } from '@noble/hashes/sha2.js';
import type { IdentityKeyPair, KeyBundle, PreKeyPair, SignedPreKey, Session } from './types.js';

const X3DH_F = new Uint8Array(32).fill(0xff);
const ZERO_SALT = new Uint8Array(32);
const X3DH_INFO = new TextEncoder().encode('YapperX3DH_v1');
const VALIDATION_SECRET = (() => {
	const secret = new Uint8Array(32);
	secret[0] = 9;
	return secret;
})();

function b64ToBytes(b64: string): Uint8Array {
	return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

function concat(...arrays: Uint8Array[]): Uint8Array {
	const total = arrays.reduce((s, a) => s + a.length, 0);
	const out = new Uint8Array(total);
	let offset = 0;
	for (const a of arrays) {
		out.set(a, offset);
		offset += a.length;
	}
	return out;
}

function isAllZero(bytes: Uint8Array): boolean {
	let diff = 0;
	for (const value of bytes) {
		diff |= value;
	}
	return diff === 0;
}

function assertValidX25519PublicKey(publicKey: Uint8Array, label: string): void {
	if (publicKey.length !== 32) {
		throw new Error(`${label} must be 32 bytes`);
	}
	try {
		const shared = x25519.getSharedSecret(VALIDATION_SECRET, publicKey);
		if (isAllZero(shared)) {
			throw new Error(`${label} produced an all-zero shared secret`);
		}
	} catch {
		throw new Error(`Invalid ${label}`);
	}
}

async function assertValidSignedPreKey(bundle: KeyBundle): Promise<void> {
	const identitySigKey = b64ToBytes(bundle.identity_sig_key);
	const signedPreKey = b64ToBytes(bundle.signed_prekey);
	const signature = b64ToBytes(bundle.signed_prekey_sig);

	if (identitySigKey.length !== 32) {
		throw new Error('Peer identity signing key must be 32 bytes');
	}
	if (signature.length !== 64) {
		throw new Error('Peer signed prekey signature must be 64 bytes');
	}

	assertValidX25519PublicKey(b64ToBytes(bundle.identity_dh_key), 'peer identity DH key');
	assertValidX25519PublicKey(signedPreKey, 'peer signed prekey');
	if (bundle.one_time_prekey) {
		assertValidX25519PublicKey(b64ToBytes(bundle.one_time_prekey), 'peer one-time prekey');
	}

	const ok = ed25519.verify(signature, signedPreKey, identitySigKey);
	if (!ok) {
		throw new Error('Peer signed prekey signature is invalid');
	}
}

function sharedSecret(privateKey: Uint8Array, publicKey: Uint8Array, label: string): Uint8Array {
	assertValidX25519PublicKey(publicKey, label);
	const shared = x25519.getSharedSecret(privateKey, publicKey);
	if (isAllZero(shared)) {
		throw new Error(`Invalid ${label}`);
	}
	return shared;
}

/**
 * Initiator side: Alice creates a session with Bob using his key bundle.
 * Returns the session, ephemeral public key, and used OPK id (for the wire message).
 */
export async function x3dhInitiate(
	myIdentity: IdentityKeyPair,
	bundle: KeyBundle,
	sessionId: string,
	conversationId: string
): Promise<{ session: Session; ephemeralPublicKey: Uint8Array; usedOpkId: number | null }> {
	await assertValidSignedPreKey(bundle);

	const ikB_dh = b64ToBytes(bundle.identity_dh_key);
	const spkB = b64ToBytes(bundle.signed_prekey);

	// Generate ephemeral key pair for this session
	const ekPriv = x25519.utils.randomSecretKey();
	const ekPub = x25519.getPublicKey(ekPriv);

	const dh1 = sharedSecret(myIdentity.dhPrivateKey, spkB, 'peer signed prekey');
	const dh2 = sharedSecret(ekPriv, ikB_dh, 'peer identity DH key');
	const dh3 = sharedSecret(ekPriv, spkB, 'peer signed prekey');

	let dhMaterial: Uint8Array;
	if (bundle.one_time_prekey) {
		const opkB = b64ToBytes(bundle.one_time_prekey);
		const dh4 = sharedSecret(ekPriv, opkB, 'peer one-time prekey');
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3, dh4);
	} else {
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3);
	}

	const sk = hkdf(sha256, dhMaterial, ZERO_SALT, X3DH_INFO, 64);
	const rootKey = sk.slice(0, 32);
	const sendChainKey = sk.slice(32, 64);

	const session: Session = {
		sessionId,
		conversationId,
		peerId: bundle.userId,
		peerDeviceId: bundle.deviceId,
		peerSignalDeviceId: bundle.signalDeviceId,
		version: 2,
		rootKey,
		sendChainKey,
		receiveChainKey: null, // set when Bob replies
		sendMsgNum: 0,
		receiveMsgNum: 0,
		myRatchetPrivKey: ekPriv,
		myRatchetPubKey: ekPub,
		peerRatchetPubKey: spkB,
		previousChainLength: 0,
		sendCount: 0,
		recvCount: 0,
		skippedMessageKeys: [],
		seenMessages: [],
	};

	return {
		session,
		ephemeralPublicKey: ekPub,
		usedOpkId: bundle.one_time_prekey_id ?? null,
	};
}

/**
 * Responder side: Bob reconstructs the session from Alice's first message.
 * Bob looks up his signed prekey and (optionally) consumed one-time prekey by id.
 */
export async function x3dhRespond(
	myIdentity: IdentityKeyPair,
	signedPreKey: SignedPreKey,
	oneTimePreKey: PreKeyPair | null,
	ephemeralPubKey: Uint8Array,
	senderDhPubKey: Uint8Array,
	sessionId: string,
	conversationId: string,
	peerId: string,
	peerDeviceId: string,
	peerSignalDeviceId: number
): Promise<Session> {
	assertValidX25519PublicKey(senderDhPubKey, 'sender identity DH key');
	assertValidX25519PublicKey(ephemeralPubKey, 'sender ephemeral key');

	const dh1 = sharedSecret(signedPreKey.privateKey, senderDhPubKey, 'sender identity DH key');
	const dh2 = sharedSecret(myIdentity.dhPrivateKey, ephemeralPubKey, 'sender ephemeral key');
	const dh3 = sharedSecret(signedPreKey.privateKey, ephemeralPubKey, 'sender ephemeral key');

	let dhMaterial: Uint8Array;
	if (oneTimePreKey) {
		const dh4 = sharedSecret(oneTimePreKey.privateKey, ephemeralPubKey, 'sender ephemeral key');
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3, dh4);
	} else {
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3);
	}

	const sk = hkdf(sha256, dhMaterial, ZERO_SALT, X3DH_INFO, 64);
	const rootKey = sk.slice(0, 32);
	const receiveChainKey = sk.slice(32, 64);

	return {
		sessionId,
		conversationId,
		peerId,
		peerDeviceId,
		peerSignalDeviceId,
		version: 2,
		rootKey,
		sendChainKey: null,
		receiveChainKey,
		sendMsgNum: 0,
		receiveMsgNum: 0,
		myRatchetPrivKey: signedPreKey.privateKey,
		myRatchetPubKey: signedPreKey.publicKey,
		peerRatchetPubKey: ephemeralPubKey,
		previousChainLength: 0,
		sendCount: 0,
		recvCount: 0,
		skippedMessageKeys: [],
		seenMessages: [],
	};
}
