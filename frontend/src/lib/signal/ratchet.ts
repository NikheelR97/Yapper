/**
 * DM ratchet support.
 *
 * `version = 1` sessions keep the legacy symmetric-only behavior for historical
 * reads. New sends use the Double Ratchet (`version = 2`) header and state.
 */

import { x25519 } from '@noble/curves/ed25519.js';
import { hkdf } from '@noble/hashes/hkdf.js';
import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';
import type { Session } from './types.js';

const MSG_KEY_INPUT = new Uint8Array([0x01]);
const CHAIN_KEY_INPUT = new Uint8Array([0x02]);
const ROOT_INFO = new TextEncoder().encode('YapperDoubleRatchetRoot_v2');
const ZERO_SALT = new Uint8Array(32);
const MAX_FORWARD_SKIP = 128;
const MAX_SKIPPED_KEYS = 512;
const MAX_SEEN_MESSAGES = 1024;

interface ModernSession extends Session {
	version: 2;
	sendChainKey: Uint8Array | null;
	receiveChainKey: Uint8Array | null;
	myRatchetPrivKey: Uint8Array;
	myRatchetPubKey: Uint8Array;
	peerRatchetPubKey: Uint8Array;
	previousChainLength: number;
	sendCount: number;
	recvCount: number;
	skippedMessageKeys: Array<{
		ratchetPub: Uint8Array;
		msgNum: number;
		messageKey: Uint8Array;
	}>;
	seenMessages: Array<{
		ratchetPub: Uint8Array;
		msgNum: number;
	}>;
}

function advanceChain(chainKey: Uint8Array): {
	messageKey: Uint8Array;
	nextChainKey: Uint8Array;
} {
	return {
		messageKey: hmac(sha256, chainKey, MSG_KEY_INPUT),
		nextChainKey: hmac(sha256, chainKey, CHAIN_KEY_INPUT),
	};
}

function legacySession(session: Session): boolean {
	return session.version !== 2;
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
	if (a.length !== b.length) {
		return false;
	}
	let diff = 0;
	for (let i = 0; i < a.length; i++) {
		diff |= a[i] ^ b[i];
	}
	return diff === 0;
}

function isAllZero(bytes: Uint8Array): boolean {
	let diff = 0;
	for (const value of bytes) {
		diff |= value;
	}
	return diff === 0;
}

function sharedSecret(privateKey: Uint8Array, publicKey: Uint8Array): Uint8Array {
	const shared = x25519.getSharedSecret(privateKey, publicKey);
	if (isAllZero(shared)) {
		throw new Error('Rejected all-zero X25519 shared secret');
	}
	return shared;
}

function deriveRootChain(
	rootKey: Uint8Array,
	dhOutput: Uint8Array
): { rootKey: Uint8Array; chainKey: Uint8Array } {
	const derived = hkdf(sha256, dhOutput, rootKey.slice(), ROOT_INFO, 64);
	return {
		rootKey: derived.slice(0, 32),
		chainKey: derived.slice(32, 64),
	};
}

function asModernSession(session: Session): ModernSession {
	if (
		session.version !== 2 ||
		!session.myRatchetPrivKey ||
		!session.myRatchetPubKey ||
		!session.peerRatchetPubKey
	) {
		throw new Error('Session is not a Double Ratchet session');
	}
	return {
		...session,
		version: 2,
		sendChainKey: session.sendChainKey ?? null,
		receiveChainKey: session.receiveChainKey ?? null,
		myRatchetPrivKey: session.myRatchetPrivKey,
		myRatchetPubKey: session.myRatchetPubKey,
		peerRatchetPubKey: session.peerRatchetPubKey,
		previousChainLength: session.previousChainLength ?? 0,
		sendCount: session.sendCount ?? 0,
		recvCount: session.recvCount ?? 0,
		skippedMessageKeys: [...(session.skippedMessageKeys ?? [])],
		seenMessages: [...(session.seenMessages ?? [])],
	};
}

function stashSkippedMessage(
	session: ModernSession,
	ratchetPub: Uint8Array,
	msgNum: number,
	messageKey: Uint8Array
): void {
	session.skippedMessageKeys.push({
		ratchetPub: ratchetPub.slice(),
		msgNum,
		messageKey: messageKey.slice(),
	});
	if (session.skippedMessageKeys.length > MAX_SKIPPED_KEYS) {
		session.skippedMessageKeys.splice(
			0,
			session.skippedMessageKeys.length - MAX_SKIPPED_KEYS
		);
	}
}

function takeSkippedMessageKey(
	session: ModernSession,
	ratchetPub: Uint8Array,
	msgNum: number
): Uint8Array | null {
	const index = session.skippedMessageKeys.findIndex(
		(entry) => entry.msgNum === msgNum && bytesEqual(entry.ratchetPub, ratchetPub)
	);
	if (index === -1) {
		return null;
	}
	const [entry] = session.skippedMessageKeys.splice(index, 1);
	return entry.messageKey;
}

function markSeen(session: ModernSession, ratchetPub: Uint8Array, msgNum: number): void {
	session.seenMessages.push({ ratchetPub: ratchetPub.slice(), msgNum });
	if (session.seenMessages.length > MAX_SEEN_MESSAGES) {
		session.seenMessages.splice(0, session.seenMessages.length - MAX_SEEN_MESSAGES);
	}
}

function hasSeen(session: ModernSession, ratchetPub: Uint8Array, msgNum: number): boolean {
	return session.seenMessages.some(
		(entry) => entry.msgNum === msgNum && bytesEqual(entry.ratchetPub, ratchetPub)
	);
}

async function encryptLegacyRatchet(
	session: Session,
	plaintext: Uint8Array
): Promise<{ ciphertext: Uint8Array; msgNum: number; updatedSession: Session }> {
	const chainKey = session.sendChainKey;
	if (!chainKey) {
		throw new Error('Legacy session is missing a send chain');
	}
	const { messageKey, nextChainKey } = advanceChain(chainKey);
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['encrypt']);
	const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, plaintext.slice());
	const ciphertext = new Uint8Array(12 + encrypted.byteLength);
	ciphertext.set(iv, 0);
	ciphertext.set(new Uint8Array(encrypted), 12);

	return {
		ciphertext,
		msgNum: session.sendMsgNum,
		updatedSession: {
			...session,
			sendChainKey: nextChainKey,
			sendMsgNum: session.sendMsgNum + 1,
		},
	};
}

async function decryptLegacyRatchet(
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

	return {
		plaintext: new Uint8Array(decrypted),
		updatedSession: {
			...session,
			receiveChainKey: nextChainKey,
			receiveMsgNum: session.receiveMsgNum + 1,
		},
	};
}

function ensureSendChain(session: ModernSession): ModernSession {
	if (session.sendChainKey) {
		return session;
	}

	const nextRatchetPriv = x25519.utils.randomSecretKey();
	const nextRatchetPub = x25519.getPublicKey(nextRatchetPriv);
	const { rootKey, chainKey } = deriveRootChain(
		session.rootKey,
		sharedSecret(nextRatchetPriv, session.peerRatchetPubKey)
	);

	return {
		...session,
		rootKey,
		sendChainKey: chainKey,
		myRatchetPrivKey: nextRatchetPriv,
		myRatchetPubKey: nextRatchetPub,
		previousChainLength: session.sendCount,
		sendCount: 0,
	};
}

function skipMessageKeys(
	session: ModernSession,
	targetMsgNum: number,
	ratchetPub: Uint8Array
): ModernSession {
	if (!session.receiveChainKey) {
		throw new Error('No receive chain key for Double Ratchet session');
	}
	if (targetMsgNum - session.recvCount > MAX_FORWARD_SKIP) {
		throw new Error('Message skipped too far ahead');
	}

	let receiveChainKey = session.receiveChainKey;
	let recvCount = session.recvCount;
	while (recvCount < targetMsgNum) {
		const { messageKey, nextChainKey } = advanceChain(receiveChainKey);
		stashSkippedMessage(session, ratchetPub, recvCount, messageKey);
		receiveChainKey = nextChainKey;
		recvCount += 1;
	}

	return {
		...session,
		receiveChainKey,
		recvCount,
		receiveMsgNum: recvCount,
	};
}

function applyReceiveRatchet(
	session: ModernSession,
	ratchetPub: Uint8Array,
	previousChainLen: number
): ModernSession {
	let nextSession = session;
	if (nextSession.receiveChainKey) {
		nextSession = skipMessageKeys(nextSession, previousChainLen, nextSession.peerRatchetPubKey);
	}

	const { rootKey, chainKey } = deriveRootChain(
		nextSession.rootKey,
		sharedSecret(nextSession.myRatchetPrivKey, ratchetPub)
	);

	return {
		...nextSession,
		rootKey,
		receiveChainKey: chainKey,
		peerRatchetPubKey: ratchetPub.slice(),
		recvCount: 0,
		receiveMsgNum: 0,
		sendChainKey: null,
	};
}

/**
 * Encrypt plaintext using the session ratchet.
 */
export async function encryptRatchet(
	session: Session,
	plaintext: Uint8Array
): Promise<{
	ciphertext: Uint8Array;
	msgNum: number;
	ratchetPub?: Uint8Array;
	previousChainLen?: number;
	cryptoVersion: number;
	updatedSession: Session;
}> {
	if (legacySession(session)) {
		const result = await encryptLegacyRatchet(session, plaintext);
		return {
			...result,
			cryptoVersion: 1,
		};
	}

	let nextSession = ensureSendChain(asModernSession(session));
	const { messageKey, nextChainKey } = advanceChain(nextSession.sendChainKey!);
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['encrypt']);
	const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, plaintext.slice());
	const ciphertext = new Uint8Array(12 + encrypted.byteLength);
	ciphertext.set(iv, 0);
	ciphertext.set(new Uint8Array(encrypted), 12);

	const msgNum = nextSession.sendCount;
	nextSession = {
		...nextSession,
		sendChainKey: nextChainKey,
		sendCount: msgNum + 1,
		sendMsgNum: msgNum + 1,
	};

	return {
		ciphertext,
		msgNum,
		ratchetPub: nextSession.myRatchetPubKey.slice(),
		previousChainLen: nextSession.previousChainLength,
		cryptoVersion: 2,
		updatedSession: nextSession,
	};
}

/**
 * Decrypt a ciphertext using the receive ratchet.
 */
export async function decryptRatchet(
	session: Session,
	ciphertext: Uint8Array,
	header?: {
		msgNum?: number;
		ratchetPub?: Uint8Array | null;
		previousChainLen?: number | null;
		cryptoVersion?: number | null;
	}
): Promise<{ plaintext: Uint8Array; updatedSession: Session }> {
	const cryptoVersion = header?.cryptoVersion ?? 1;
	if (cryptoVersion === 1 || legacySession(session)) {
		return decryptLegacyRatchet(session, ciphertext);
	}

	const ratchetPub = header?.ratchetPub;
	const msgNum = header?.msgNum;
	if (!ratchetPub || msgNum == null) {
		throw new Error('Double Ratchet header is incomplete');
	}

	let nextSession = asModernSession(session);
	if (hasSeen(nextSession, ratchetPub, msgNum)) {
		throw new Error('Replay detected for Double Ratchet message');
	}

	let messageKey = takeSkippedMessageKey(nextSession, ratchetPub, msgNum);
	if (!messageKey) {
		if (!bytesEqual(ratchetPub, nextSession.peerRatchetPubKey)) {
			nextSession = applyReceiveRatchet(
				nextSession,
				ratchetPub,
				header?.previousChainLen ?? 0
			);
		}

		nextSession = skipMessageKeys(nextSession, msgNum, ratchetPub);
		if (!nextSession.receiveChainKey) {
			throw new Error('No receive chain key after ratchet step');
		}
		const derived = advanceChain(nextSession.receiveChainKey);
		messageKey = derived.messageKey;
		nextSession = {
			...nextSession,
			receiveChainKey: derived.nextChainKey,
			recvCount: msgNum + 1,
			receiveMsgNum: msgNum + 1,
		};
	}

	const iv = ciphertext.slice(0, 12);
	const data = ciphertext.slice(12);
	const aesKey = await crypto.subtle.importKey('raw', messageKey.slice(), 'AES-GCM', false, ['decrypt']);
	const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, data);
	markSeen(nextSession, ratchetPub, msgNum);

	return {
		plaintext: new Uint8Array(decrypted),
		updatedSession: nextSession,
	};
}
