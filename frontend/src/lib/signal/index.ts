/**
 * Signal Protocol wrapper — main API.
 *
 * Uses @noble/curves (Ed25519 + X25519) and Web Crypto AES-256-GCM.
 * Works in all WebView environments: Tauri, Capacitor, and Web PWA.
 *
 * Typical call order:
 *   1. generateIdentityKey()       — on first account setup
 *   2. generateSignedPreKey(id)    — on setup + rotate weekly
 *   3. generateOneTimePreKeys(100) — on setup + refill when low
 *   4. uploadKeysToServer()        — after generating
 *   5. encryptDm() / decryptDm()   — for each DM message
 *   6. joinChannel(channelId)      — on first join (generates + distributes SenderKey)
 *   7. encryptChannel() / decryptChannel() — for channel messages
 */

import { x25519, ed25519 } from '@noble/curves/ed25519.js';
import { api } from '$api/client.js';
import * as ks from './keystore.js';
import { x3dhInitiate, x3dhRespond } from './x3dh.js';
import { encryptRatchet, decryptRatchet } from './ratchet.js';
import {
	generateSenderKey,
	encryptWithSenderKey,
	decryptWithSenderKey,
	encryptSenderKeyDist,
	decryptSenderKeyDist,
	packChannelMessage,
	unpackChannelMessage,
} from './sender_keys.js';
import type {
	IdentityKeyPair,
	KeyBundle,
	PreKeyPair,
	SignedPreKey,
	Session,
	EncryptedMessage,
	SenderKey,
	SenderKeyRecord,
	SenderKeyDistPayload,
	EncryptedChannelMessage,
} from './types.js';

export type {
	IdentityKeyPair, KeyBundle, PreKeyPair, SignedPreKey, Session, EncryptedMessage,
	SenderKey, SenderKeyRecord, EncryptedChannelMessage,
};
export { backupKeys, restoreKeys } from './backup.js';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function bytesToB64(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes));
}

function b64ToBytes(b64: string): Uint8Array {
	return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

// ─── Key Generation ───────────────────────────────────────────────────────────

/**
 * Generate a fresh identity key pair.
 * Call once on first device registration; persist via keystore.
 */
export async function generateIdentityKey(): Promise<IdentityKeyPair> {
	const dhPrivateKey = x25519.utils.randomSecretKey();
	const dhPublicKey = x25519.getPublicKey(dhPrivateKey);
	const sigPrivateKey = ed25519.utils.randomSecretKey();
	const sigPublicKey = ed25519.getPublicKey(sigPrivateKey);
	return { dhPublicKey, dhPrivateKey, sigPublicKey, sigPrivateKey };
}

/**
 * Generate a signed prekey.
 * @param keyId  — monotonic integer, e.g. 1 for first key
 */
export async function generateSignedPreKey(
	identity: IdentityKeyPair,
	keyId: number
): Promise<SignedPreKey> {
	const privateKey = x25519.utils.randomSecretKey();
	const publicKey = x25519.getPublicKey(privateKey);
	// Sign the X25519 public key with the Ed25519 identity signing key
	const signature = ed25519.sign(publicKey, identity.sigPrivateKey);
	return { keyId, publicKey, privateKey, signature, createdAt: Date.now() };
}

/**
 * Generate a batch of one-time prekeys.
 * @param count   — number of keys (default 100)
 * @param startId — first key id in the batch
 */
export async function generateOneTimePreKeys(
	count = 100,
	startId = 1
): Promise<PreKeyPair[]> {
	return Array.from({ length: count }, (_, i) => {
		const privateKey = x25519.utils.randomSecretKey();
		const publicKey = x25519.getPublicKey(privateKey);
		return { keyId: startId + i, publicKey, privateKey };
	});
}

// ─── Key Upload ───────────────────────────────────────────────────────────────

/** Upload identity key to the server. Call once after generateIdentityKey(). */
export async function uploadIdentityKey(identity: IdentityKeyPair): Promise<void> {
	await api.post('/api/v1/keys/identity', {
		device_id: 1,
		dh_public_key: bytesToB64(identity.dhPublicKey),
		signing_public_key: bytesToB64(identity.sigPublicKey),
	});
}

/** Upload a signed prekey to the server. */
export async function uploadSignedPreKey(spk: SignedPreKey): Promise<void> {
	await api.post('/api/v1/keys/signed-prekey', {
		device_id: 1,
		key_id: spk.keyId,
		public_key: bytesToB64(spk.publicKey),
		signature: bytesToB64(spk.signature),
	});
}

/** Upload a batch of one-time prekeys to the server. */
export async function uploadOneTimePreKeys(prekeys: PreKeyPair[]): Promise<void> {
	await api.post('/api/v1/keys/one-time-prekeys', {
		device_id: 1,
		keys: prekeys.map((k) => ({
			key_id: k.keyId,
			public_key: bytesToB64(k.publicKey),
		})),
	});
}

// ─── Full Setup ───────────────────────────────────────────────────────────────

/**
 * First-time key setup: generate all keys, persist locally, upload to server.
 * Idempotent — if keys already exist in the keystore, this is a no-op.
 */
export async function setupKeys(): Promise<void> {
	let identity = await ks.loadIdentityKeyPair();
	if (identity) return; // Already set up

	identity = await generateIdentityKey();
	await ks.storeIdentityKeyPair(identity);

	const spk = await generateSignedPreKey(identity, 1);
	await ks.storeSignedPreKey(spk);

	const prekeys = await generateOneTimePreKeys(100, 1);
	for (const pk of prekeys) {
		await ks.storePreKey(pk);
	}

	// Upload all keys to server
	await uploadIdentityKey(identity);
	await uploadSignedPreKey(spk);
	await uploadOneTimePreKeys(prekeys);
}

/**
 * Check OPK count on server and replenish if below threshold.
 * Call after setupKeys() on each app start.
 */
export async function replenishPreKeysIfNeeded(): Promise<void> {
	const { count, low } = await api.get<{ count: number; low: boolean }>(
		'/api/v1/keys/one-time-prekey-count'
	);
	if (!low) return;

	const identity = await ks.loadIdentityKeyPair();
	if (!identity) return;

	const existing = await ks.countPreKeys();
	const startId = existing + 1;
	const newKeys = await generateOneTimePreKeys(100, startId);
	for (const pk of newKeys) {
		await ks.storePreKey(pk);
	}
	await uploadOneTimePreKeys(newKeys);
	console.debug(`[Signal] Replenished ${newKeys.length} OPKs (was ${count})`);
}

// ─── Encrypt DM ───────────────────────────────────────────────────────────────

/**
 * Encrypt a plaintext message for a DM conversation.
 * If no session exists yet, initiates X3DH and returns ephemeral key info.
 */
export async function encryptDm(
	conversationId: string,
	peerId: string,
	plaintext: string
): Promise<EncryptedMessage> {
	let session = await ks.loadSession(conversationId);

	let ephemeralKey: string | undefined;
	let opkId: number | undefined;

	if (!session) {
		// No session — initiate X3DH
		const identity = await ks.loadIdentityKeyPair();
		if (!identity) throw new Error('No identity key — call setupKeys() first');

		const bundle = await api.get<KeyBundle>(`/api/v1/keys/${peerId}`);
		const result = await x3dhInitiate(identity, bundle, conversationId);

		session = result.session;
		ephemeralKey = bytesToB64(result.ephemeralPublicKey);
		opkId = result.usedOpkId ?? undefined;
	}

	const plainBytes = new TextEncoder().encode(plaintext);
	const { ciphertext, msgNum, updatedSession } = await encryptRatchet(session, plainBytes);

	await ks.storeSession(updatedSession);

	return {
		ciphertext: bytesToB64(ciphertext),
		ephemeralKey,
		opkId,
		msgNum,
	};
}

// ─── Decrypt DM ───────────────────────────────────────────────────────────────

/**
 * Decrypt a received DM message.
 * On first message from a peer, reconstructs the X3DH session (responder side).
 */
export async function decryptDm(
	conversationId: string,
	peerId: string,
	msg: {
		ciphertext: string;
		ephemeral_key?: string | null;
		opk_id?: number | null;
	}
): Promise<string> {
	let session = await ks.loadSession(conversationId);
	const cipherBytes = b64ToBytes(msg.ciphertext);

	if (!session) {
		// First message — perform X3DH responder role
		if (!msg.ephemeral_key) {
			throw new Error('No session and no ephemeral key — cannot decrypt');
		}

		const identity = await ks.loadIdentityKeyPair();
		if (!identity) throw new Error('No identity key');

		const spk = await ks.loadLatestSignedPreKey();
		if (!spk) throw new Error('No signed prekey');

		const opk = msg.opk_id != null ? await ks.loadPreKey(msg.opk_id) : null;
		const ephemeralPubKey = b64ToBytes(msg.ephemeral_key);

		// Fetch sender's DH public key from server to use in X3DH
		const senderBundle = await api.get<KeyBundle>(`/api/v1/keys/${peerId}`);
		const senderDhPub = b64ToBytes(senderBundle.identity_dh_key);

		session = await x3dhRespond(
			identity,
			spk,
			opk,
			ephemeralPubKey,
			senderDhPub,
			conversationId,
			peerId
		);

		// Delete consumed OPK
		if (msg.opk_id != null) {
			await ks.deletePreKey(msg.opk_id);
		}
	}

	const { plaintext, updatedSession } = await decryptRatchet(session, cipherBytes);
	await ks.storeSession(updatedSession);

	return new TextDecoder().decode(plaintext);
}

// ─── Channel (group) E2EE ─────────────────────────────────────────────────────

/**
 * Encrypt a message for a channel using the Sender Key ratchet.
 * Returns the packed wire ciphertext (sig || ct, base64) and the ratchet iteration.
 * Throws if no SenderKey exists for this channel — call joinChannel() first.
 */
export async function encryptChannel(
	channelId: string,
	plaintext: string
): Promise<{ wireCiphertext: string; iteration: number }> {
	const key = await ks.loadSenderKey(channelId);
	if (!key) throw new Error(`No SenderKey for channel ${channelId} — call joinChannel() first`);

	const plainBytes = new TextEncoder().encode(plaintext);
	const { encrypted, updatedKey } = await encryptWithSenderKey(key, plainBytes);
	await ks.storeSenderKey(updatedKey);
	return { wireCiphertext: packChannelMessage(encrypted), iteration: encrypted.iteration };
}

/**
 * Decrypt a received channel message from the wire format.
 * The ciphertext field must be base64(sig_64_bytes || aes_ct_bytes) as packed by encryptChannel.
 * Throws if no SenderKeyRecord exists for (channelId, senderId).
 */
export async function decryptChannel(
	channelId: string,
	senderId: string,
	msg: { ciphertext: string; msg_num: number | null }
): Promise<string> {
	const record = await ks.loadReceiverKey(channelId, senderId);
	if (!record) throw new Error(`No SenderKey for sender ${senderId} in channel ${channelId}`);

	const encrypted = unpackChannelMessage(msg.ciphertext, msg.msg_num ?? 0);
	const { plaintext, updatedRecord } = await decryptWithSenderKey(record, encrypted);
	await ks.storeReceiverKey(updatedRecord);
	return new TextDecoder().decode(plaintext);
}

/**
 * Join a channel: generate a SenderKey, distribute it to all existing members,
 * and fetch any pending SenderKey distributions from other members.
 *
 * Call this once when the user first joins (or after a server restart wipes local keys).
 */
export async function joinChannel(channelId: string): Promise<void> {
	const identity = await ks.loadIdentityKeyPair();
	if (!identity) throw new Error('No identity key — call setupKeys() first');

	// Generate and persist our SenderKey for this channel
	const senderKey = generateSenderKey(channelId);
	await ks.storeSenderKey(senderKey);

	// Fetch all channel members
	const members = await api.get<Array<{ user_id: string; username: string }>>(
		`/api/v1/channels/${channelId}/members`
	);

	// Build the distribution payload
	const distPayload: SenderKeyDistPayload = {
		channelId,
		chainKey: bytesToB64(senderKey.chainKey),
		signingPubKey: bytesToB64(senderKey.signingPubKey),
		iteration: senderKey.iteration,
	};

	// Resolve own user_id once — used to skip self in the distribution loop
	const me = await api.get<{ user_id: string }>('/api/v1/users/me');
	const myUserId = me.user_id;

	// Encrypt for each member (except ourselves) using ECIES
	const distributions: Array<{ to_user_id: string; ciphertext: string; ek_public: string }> = [];

	for (const member of members) {
		if (member.user_id === myUserId) continue;

		// Fetch recipient's DH public key
		let recipientBundle: KeyBundle;
		try {
			recipientBundle = await api.get<KeyBundle>(`/api/v1/keys/${member.user_id}`);
		} catch {
			console.warn(`[Signal] Could not fetch key bundle for ${member.user_id} — skipping`);
			continue;
		}

		const recipientDhPub = b64ToBytes(recipientBundle.identity_dh_key);
		const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(distPayload, recipientDhPub);

		distributions.push({
			to_user_id: member.user_id,
			ciphertext: bytesToB64(ciphertext),
			ek_public: bytesToB64(ephemeralKey),
		});
	}

	if (distributions.length > 0) {
		await api.post(`/api/v1/channels/${channelId}/sender-key-dist`, { distributions });
	}

	// Fetch pending distributions addressed to us (from members who joined before us)
	await fetchAndStorePendingDists(channelId, identity);
}

/**
 * Fetch any pending SenderKey distributions for this channel and persist them.
 * Call on channel open to catch distributions delivered while offline.
 */
export async function fetchPendingKeyDists(channelId: string): Promise<void> {
	const identity = await ks.loadIdentityKeyPair();
	if (!identity) return;
	await fetchAndStorePendingDists(channelId, identity);
}

async function fetchAndStorePendingDists(
	channelId: string,
	identity: IdentityKeyPair
): Promise<void> {
	const dists = await api
		.get<Array<{ from_user: string; ciphertext: string; ek_public: string }>>(
			`/api/v1/channels/${channelId}/sender-key-dist`
		)
		.catch(() => [] as Array<{ from_user: string; ciphertext: string; ek_public: string }>);

	for (const dist of dists) {
		try {
			const ct = b64ToBytes(dist.ciphertext);
			const ek = b64ToBytes(dist.ek_public);
			const payload = await decryptSenderKeyDist(ct, ek, identity.dhPrivateKey, identity.dhPublicKey);
			const record: SenderKeyRecord = {
				channelId: payload.channelId,
				senderId: dist.from_user,
				chainKey: b64ToBytes(payload.chainKey),
				signingPubKey: b64ToBytes(payload.signingPubKey),
				iteration: payload.iteration,
			};
			await ks.storeReceiverKey(record);
		} catch (e) {
			console.warn(`[Signal] Failed to decrypt SenderKey dist from ${dist.from_user}:`, e);
		}
	}
}

/**
 * Prepare a channel for messaging (idempotent).
 * - Generates and distributes a SenderKey if none exists locally yet.
 * - Fetches any pending SenderKey distributions from other members regardless.
 * Safe to call on every channel open — will not regenerate if key already exists.
 */
export async function prepareChannel(channelId: string): Promise<void> {
	const existing = await ks.loadSenderKey(channelId);
	if (!existing) {
		await joinChannel(channelId);
	} else {
		await fetchPendingKeyDists(channelId);
	}
}

/**
 * Handle a real-time key_dist WS event.
 * Decrypts the SenderKey distribution and stores it immediately.
 */
export async function receiveSenderKeyDist(event: {
	channel_id: string;
	from_user: string;
	ciphertext: string;
	ek_public: string;
}): Promise<void> {
	const identity = await ks.loadIdentityKeyPair();
	if (!identity) return;

	try {
		const ct = b64ToBytes(event.ciphertext);
		const ek = b64ToBytes(event.ek_public);
		const payload = await decryptSenderKeyDist(ct, ek, identity.dhPrivateKey, identity.dhPublicKey);
		const record: SenderKeyRecord = {
			channelId: payload.channelId,
			senderId: event.from_user,
			chainKey: b64ToBytes(payload.chainKey),
			signingPubKey: b64ToBytes(payload.signingPubKey),
			iteration: payload.iteration,
		};
		await ks.storeReceiverKey(record);
		console.debug(`[Signal] Received SenderKey from ${event.from_user} for channel ${event.channel_id}`);
	} catch (e) {
		console.warn(`[Signal] Failed to process key_dist from ${event.from_user}:`, e);
	}
}
