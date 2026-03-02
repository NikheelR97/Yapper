/**
 * Symmetric ratchet for message key derivation.
 *
 * Each message consumes one step of the chain key, producing a unique
 * message key used for AES-256-GCM encryption.
 *
 * Chain key advancement (Signal spec §3.2):
 *   message_key  = HMAC-SHA256(chain_key, 0x01)
 *   next_chain   = HMAC-SHA256(chain_key, 0x02)
 *
 * Encryption: AES-256-GCM with a random 12-byte IV prepended to the ciphertext.
 */

import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';
import type { Session } from './types.js';

const MSG_KEY_INPUT = new Uint8Array([0x01]);
const CHAIN_KEY_INPUT = new Uint8Array([0x02]);

function advanceChain(chainKey: Uint8Array): {
	messageKey: Uint8Array;
	nextChainKey: Uint8Array;
} {
	const messageKey = hmac(sha256, chainKey, MSG_KEY_INPUT);
	const nextChainKey = hmac(sha256, chainKey, CHAIN_KEY_INPUT);
	return { messageKey, nextChainKey };
}

/**
 * Encrypt plaintext using the send ratchet.
 * Returns the updated session and the AES-256-GCM ciphertext (IV prepended).
 */
export async function encryptRatchet(
	session: Session,
	plaintext: Uint8Array
): Promise<{ ciphertext: Uint8Array; msgNum: number; updatedSession: Session }> {
	const { messageKey, nextChainKey } = advanceChain(session.sendChainKey);
	const iv = crypto.getRandomValues(new Uint8Array(12));
	// .slice() gives Uint8Array<ArrayBuffer> which satisfies BufferSource for Web Crypto
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['encrypt']);
	const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, plaintext.slice());

	// Prepend IV so the receiver can extract it
	const ciphertext = new Uint8Array(12 + encrypted.byteLength);
	ciphertext.set(iv, 0);
	ciphertext.set(new Uint8Array(encrypted), 12);

	const updatedSession: Session = {
		...session,
		sendChainKey: nextChainKey,
		sendMsgNum: session.sendMsgNum + 1,
	};

	return { ciphertext, msgNum: session.sendMsgNum, updatedSession };
}

/**
 * Decrypt a ciphertext using the receive ratchet.
 * Returns the plaintext and updated session.
 */
export async function decryptRatchet(
	session: Session,
	ciphertext: Uint8Array
): Promise<{ plaintext: Uint8Array; updatedSession: Session }> {
	if (!session.receiveChainKey) {
		throw new Error('No receive chain key — session not yet established from peer');
	}

	const { messageKey, nextChainKey } = advanceChain(session.receiveChainKey);
	const iv = ciphertext.slice(0, 12);
	const data = ciphertext.slice(12);

	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['decrypt']);
	const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, data);

	const updatedSession: Session = {
		...session,
		receiveChainKey: nextChainKey,
		receiveMsgNum: session.receiveMsgNum + 1,
	};

	return { plaintext: new Uint8Array(decrypted), updatedSession };
}
