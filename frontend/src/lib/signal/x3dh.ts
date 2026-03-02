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

import { x25519 } from '@noble/curves/ed25519.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { sha256 } from '@noble/hashes/sha2.js';
import type { IdentityKeyPair, KeyBundle, PreKeyPair, SignedPreKey, Session } from './types.js';

const X3DH_F = new Uint8Array(32).fill(0xff);
const X3DH_INFO = new TextEncoder().encode('YapperX3DH_v1');

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

/**
 * Initiator side: Alice creates a session with Bob using his key bundle.
 * Returns the session, ephemeral public key, and used OPK id (for the wire message).
 */
export async function x3dhInitiate(
	myIdentity: IdentityKeyPair,
	bundle: KeyBundle,
	conversationId: string
): Promise<{ session: Session; ephemeralPublicKey: Uint8Array; usedOpkId: number | null }> {
	const ikB_dh = b64ToBytes(bundle.identity_dh_key);
	const spkB = b64ToBytes(bundle.signed_prekey);

	// Generate ephemeral key pair for this session
	const ekPriv = x25519.utils.randomSecretKey();
	const ekPub = x25519.getPublicKey(ekPriv);

	const dh1 = x25519.getSharedSecret(myIdentity.dhPrivateKey, spkB);
	const dh2 = x25519.getSharedSecret(ekPriv, ikB_dh);
	const dh3 = x25519.getSharedSecret(ekPriv, spkB);

	let dhMaterial: Uint8Array;
	if (bundle.one_time_prekey) {
		const opkB = b64ToBytes(bundle.one_time_prekey);
		const dh4 = x25519.getSharedSecret(ekPriv, opkB);
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3, dh4);
	} else {
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3);
	}

	const sk = hkdf(sha256, dhMaterial, undefined, X3DH_INFO, 64);
	const rootKey = sk.slice(0, 32);
	const sendChainKey = sk.slice(32, 64);

	const session: Session = {
		conversationId,
		peerId: bundle.userId,
		rootKey,
		sendChainKey,
		receiveChainKey: null, // set when Bob replies
		sendMsgNum: 0,
		receiveMsgNum: 0,
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
	conversationId: string,
	peerId: string
): Promise<Session> {
	const dh1 = x25519.getSharedSecret(signedPreKey.privateKey, senderDhPubKey);
	const dh2 = x25519.getSharedSecret(myIdentity.dhPrivateKey, ephemeralPubKey);
	const dh3 = x25519.getSharedSecret(signedPreKey.privateKey, ephemeralPubKey);

	let dhMaterial: Uint8Array;
	if (oneTimePreKey) {
		const dh4 = x25519.getSharedSecret(oneTimePreKey.privateKey, ephemeralPubKey);
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3, dh4);
	} else {
		dhMaterial = concat(X3DH_F, dh1, dh2, dh3);
	}

	const sk = hkdf(sha256, dhMaterial, undefined, X3DH_INFO, 64);
	const rootKey = sk.slice(0, 32);
	const receiveChainKey = sk.slice(32, 64);

	// Bob's initial send chain key will be set when he first sends a message.
	// For now, derive it from the root key so encryption can start immediately.
	const sendChainKey = hkdf(sha256, rootKey, undefined, new TextEncoder().encode('send'), 32);

	return {
		conversationId,
		peerId,
		rootKey,
		sendChainKey,
		receiveChainKey,
		sendMsgNum: 0,
		receiveMsgNum: 0,
	};
}
