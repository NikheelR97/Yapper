/**
 * Sender Key protocol for group E2EE (channel messages).
 *
 * Each user has one SenderKey per channel, consisting of:
 *   - A chain key (ratcheted per message, same HMAC-SHA256 scheme as the DM ratchet)
 *   - An Ed25519 signing key pair (authenticates messages — prevents server from forging)
 *
 * Key distribution uses ECIES (one-shot, no session state):
 *   1. Generate ephemeral X25519 key pair
 *   2. DH(ek_priv, recipient_dh_pub) → shared secret
 *   3. HKDF(shared || ek_pub || recipient_pub, "SenderKeyDist_v1") → enc_key
 *   4. AES-256-GCM encrypt the SenderKeyDistPayload JSON
 *
 * Encryption of channel messages:
 *   1. Advance chain key: msg_key = HMAC(chain_key, 0x01), next_chain = HMAC(chain_key, 0x02)
 *   2. AES-256-GCM encrypt plaintext with msg_key (random IV, prepended to ciphertext)
 *   3. Ed25519 sign (channelId_bytes || iteration_le32 || ciphertext)
 */

import { x25519, ed25519 } from '@noble/curves/ed25519.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';
import type { SenderKey, SenderKeyRecord, SenderKeyDistPayload, EncryptedChannelMessage } from './types.js';

export type { SenderKey, SenderKeyRecord, SenderKeyDistPayload, EncryptedChannelMessage };

// ─── Constants ────────────────────────────────────────────────────────────────

const MSG_KEY_INPUT   = new Uint8Array([0x01]);
const CHAIN_KEY_INPUT = new Uint8Array([0x02]);
const DIST_INFO = new TextEncoder().encode('SenderKeyDist_v1');

// ─── Helpers ──────────────────────────────────────────────────────────────────

function concat(...arrays: Uint8Array[]): Uint8Array {
	const total = arrays.reduce((s, a) => s + a.length, 0);
	const out = new Uint8Array(total);
	let offset = 0;
	for (const a of arrays) { out.set(a, offset); offset += a.length; }
	return out;
}

function iterationBytes(n: number): Uint8Array {
	const buf = new Uint8Array(4);
	new DataView(buf.buffer).setUint32(0, n, true); // little-endian
	return buf;
}

// ─── Chain key ratchet ────────────────────────────────────────────────────────

function advanceChain(chainKey: Uint8Array): { messageKey: Uint8Array; nextChainKey: Uint8Array } {
	return {
		messageKey:   hmac(sha256, chainKey, MSG_KEY_INPUT),
		nextChainKey: hmac(sha256, chainKey, CHAIN_KEY_INPUT),
	};
}

// ─── Key generation ───────────────────────────────────────────────────────────

/** Generate a fresh SenderKey for a channel. */
export function generateSenderKey(channelId: string): SenderKey {
	const chainKey      = crypto.getRandomValues(new Uint8Array(32));
	const signingPrivKey = ed25519.utils.randomSecretKey();
	const signingPubKey  = ed25519.getPublicKey(signingPrivKey);
	return { channelId, chainKey, signingPubKey, signingPrivKey, iteration: 0 };
}

// ─── Channel message encryption ───────────────────────────────────────────────

/**
 * Encrypt a plaintext message using the SenderKey ratchet.
 * Returns the encrypted message and the updated key (caller must persist the key).
 */
export async function encryptWithSenderKey(
	key: SenderKey,
	plaintext: Uint8Array,
): Promise<{ encrypted: EncryptedChannelMessage; updatedKey: SenderKey }> {
	const { messageKey, nextChainKey } = advanceChain(key.chainKey);

	// AES-256-GCM
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['encrypt']);
	const ct = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, plaintext.slice());

	// iv || ciphertext
	const ciphertextBytes = concat(iv, new Uint8Array(ct));
	const ciphertextB64 = bytesToB64(ciphertextBytes);

	// Ed25519 sign: channelId_utf8 || iteration_le32 || ciphertext_bytes
	const sigInput = concat(
		new TextEncoder().encode(key.channelId),
		iterationBytes(key.iteration),
		ciphertextBytes,
	);
	const signature = bytesToB64(ed25519.sign(sigInput, key.signingPrivKey));

	const updatedKey: SenderKey = {
		...key,
		chainKey: nextChainKey,
		iteration: key.iteration + 1,
	};

	return {
		encrypted: { ciphertext: ciphertextB64, signature, iteration: key.iteration },
		updatedKey,
	};
}

// ─── Channel message decryption ───────────────────────────────────────────────

/**
 * Decrypt a received channel message.
 * Advances the receiver's chain key to match the message iteration.
 * Saves message keys for skipped messages (up to MAX_SKIP steps ahead).
 */
const MAX_SKIP = 100;
const MAX_HISTORY_SKIP = 100_000;

function deriveForIteration(
	startChainKey: Uint8Array,
	startIteration: number,
	targetIteration: number
): { messageKey: Uint8Array; nextChainKey: Uint8Array } {
	let chainKey = startChainKey;
	let messageKey: Uint8Array | null = null;
	for (let i = startIteration; i <= targetIteration; i++) {
		const { messageKey: mk, nextChainKey } = advanceChain(chainKey);
		if (i === targetIteration) messageKey = mk;
		chainKey = nextChainKey;
	}
	if (!messageKey) throw new Error('[SenderKey] Failed to derive message key');
	return { messageKey, nextChainKey: chainKey };
}

export async function decryptWithSenderKey(
	record: SenderKeyRecord,
	encrypted: EncryptedChannelMessage,
	opts: { allowHistorical?: boolean } = {}
): Promise<{ plaintext: Uint8Array; updatedRecord: SenderKeyRecord }> {
	const { iteration } = encrypted;
	const allowHistorical = opts.allowHistorical === true;

	let messageKey: Uint8Array;
	let nextChainKey = record.chainKey;
	let nextIteration = record.iteration;

	if (iteration < record.iteration) {
		if (!allowHistorical) {
			throw new Error(`[SenderKey] Message iteration ${iteration} already consumed (at ${record.iteration})`);
		}

		const initialChainKey = record.initialChainKey;
		const initialIteration = record.initialIteration ?? 0;
		if (!initialChainKey || iteration < initialIteration) {
			throw new Error(`[SenderKey] No historical key material for iteration ${iteration}`);
		}
		if (iteration - initialIteration > MAX_HISTORY_SKIP) {
			throw new Error(
				`[SenderKey] Historical iteration ${iteration} too far from seed (at ${initialIteration})`
			);
		}

		const derived = deriveForIteration(initialChainKey, initialIteration, iteration);
		messageKey = derived.messageKey;
		// Keep current ratchet state intact when decrypting old history.
		nextChainKey = record.chainKey;
		nextIteration = record.iteration;
	} else {
		if (iteration - record.iteration > MAX_SKIP) {
			throw new Error(`[SenderKey] Message iteration ${iteration} too far ahead (at ${record.iteration})`);
		}

		const derived = deriveForIteration(record.chainKey, record.iteration, iteration);
		messageKey = derived.messageKey;
		nextChainKey = derived.nextChainKey;
		nextIteration = iteration + 1;
	}

	// Verify Ed25519 signature
	const ciphertextBytes = b64ToBytes(encrypted.ciphertext);
	const sigInput = concat(
		new TextEncoder().encode(record.channelId),
		iterationBytes(iteration),
		ciphertextBytes,
	);
	const validSig = ed25519.verify(b64ToBytes(encrypted.signature), sigInput, record.signingPubKey);
	if (!validSig) {
		throw new Error('[SenderKey] Invalid message signature — possible forgery');
	}

	// AES-256-GCM decrypt (iv is first 12 bytes)
	const iv   = ciphertextBytes.slice(0, 12);
	const data = ciphertextBytes.slice(12);
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['decrypt']);
	const plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, data);

	const updatedRecord: SenderKeyRecord = {
		...record,
		chainKey: nextChainKey,
		iteration: nextIteration,
	};

	return { plaintext: new Uint8Array(plain), updatedRecord };
}

// ─── ECIES key distribution ───────────────────────────────────────────────────

/**
 * ECIES-encrypt a SenderKeyDistPayload for a single recipient.
 *
 * Wire format: ek_pub(32) || iv(12) || aes_gcm_ciphertext
 * (ek_pub is sent separately as `ek_public` field — here we return both)
 */
export async function encryptSenderKeyDist(
	payload: SenderKeyDistPayload,
	recipientDhPub: Uint8Array,
): Promise<{ ciphertext: Uint8Array; ephemeralKey: Uint8Array }> {
	const ekPriv = x25519.utils.randomSecretKey();
	const ekPub  = x25519.getPublicKey(ekPriv);

	const shared = x25519.getSharedSecret(ekPriv, recipientDhPub);

	// Bind both public keys to prevent key-compromise impersonation
	const ikm = concat(shared, ekPub, recipientDhPub);
	const encKey = hkdf(sha256, ikm, undefined, DIST_INFO, 32);

	const plaintext = new TextEncoder().encode(JSON.stringify(payload));
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const aesKey = await crypto.subtle.importKey('raw', encKey.slice(), 'AES-GCM', false, ['encrypt']);
	const ct = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, plaintext.slice());

	const ciphertext = concat(iv, new Uint8Array(ct));
	return { ciphertext, ephemeralKey: ekPub };
}

/**
 * ECIES-decrypt a received SenderKey distribution.
 */
export async function decryptSenderKeyDist(
	ciphertext: Uint8Array,
	ephemeralKeyPub: Uint8Array,
	myDhPriv: Uint8Array,
	myDhPub: Uint8Array,
): Promise<SenderKeyDistPayload> {
	const shared = x25519.getSharedSecret(myDhPriv, ephemeralKeyPub);
	const ikm = concat(shared, ephemeralKeyPub, myDhPub);
	const encKey = hkdf(sha256, ikm, undefined, DIST_INFO, 32);

	const iv   = ciphertext.slice(0, 12);
	const data = ciphertext.slice(12);
	const aesKey = await crypto.subtle.importKey('raw', encKey.slice(), 'AES-GCM', false, ['decrypt']);
	const plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, data);

	return JSON.parse(new TextDecoder().decode(plain)) as SenderKeyDistPayload;
}

// ─── Wire format helpers ──────────────────────────────────────────────────────

/**
 * Pack an EncryptedChannelMessage into a single base64 string for wire transport.
 * Wire format: sig(64 bytes) || aes_ct(variable)
 * Ed25519 signatures are always 64 bytes, so splitting on receive is deterministic.
 */
export function packChannelMessage(msg: EncryptedChannelMessage): string {
	const sig = b64ToBytes(msg.signature);
	const ct  = b64ToBytes(msg.ciphertext);
	return bytesToB64(concat(sig, ct));
}

/**
 * Unpack a wire-format ciphertext back into an EncryptedChannelMessage.
 */
export function unpackChannelMessage(wireCt: string, iteration: number): EncryptedChannelMessage {
	const bytes = b64ToBytes(wireCt);
	return {
		signature:  bytesToB64(bytes.slice(0, 64)),
		ciphertext: bytesToB64(bytes.slice(64)),
		iteration,
	};
}

// ─── Encoding helpers ─────────────────────────────────────────────────────────

function bytesToB64(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes));
}

function b64ToBytes(b64: string): Uint8Array {
	return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}
