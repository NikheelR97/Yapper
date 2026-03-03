/**
 * Persistent key storage using IndexedDB (via `idb`).
 *
 * Works in all WebView environments (Tauri, Capacitor, Web PWA).
 * Tauri Stronghold integration is deferred to a later sprint.
 *
 * Store layout:
 *   identity       — singleton IdentityKeyPair (keyed by 'own')
 *   prekeys        — Map<keyId, PreKeyPair>
 *   signed_prekeys — Map<keyId, SignedPreKey>
 *   sessions       — Map<conversationId, Session>
 *   sender_keys    — Map<channelId, SenderKey>        (v2, our own key per channel)
 *   receiver_keys  — Map<channelId:senderId, SenderKeyRecord>  (v2, received keys)
 */

import { openDB, type IDBPDatabase } from 'idb';
import type {
	IdentityKeyPair,
	PreKeyPair,
	SignedPreKey,
	Session,
	SenderKey,
	SenderKeyRecord,
} from './types.js';

const DB_NAME = 'yapper-signal';
const DB_VERSION = 2;

let _db: IDBPDatabase | null = null;
let _dbPromise: Promise<IDBPDatabase> | null = null;

async function getDB(): Promise<IDBPDatabase> {
	if (_db) return _db;
	if (!_dbPromise) {
		_dbPromise = openDB(DB_NAME, DB_VERSION, {
			upgrade(db, oldVersion) {
				// v1 stores (always create if missing on fresh install)
				if (!db.objectStoreNames.contains('identity'))       db.createObjectStore('identity');
				if (!db.objectStoreNames.contains('prekeys'))        db.createObjectStore('prekeys', { keyPath: 'keyId' });
				if (!db.objectStoreNames.contains('signed_prekeys')) db.createObjectStore('signed_prekeys', { keyPath: 'keyId' });
				if (!db.objectStoreNames.contains('sessions'))       db.createObjectStore('sessions', { keyPath: 'conversationId' });
				// v2 — Sender Keys for group E2EE
				if (oldVersion < 2) {
					db.createObjectStore('sender_keys', { keyPath: 'channelId' });
					db.createObjectStore('receiver_keys'); // keyed externally as `${channelId}:${senderId}`
				}
			},
		}).then((db) => {
			_db = db;
			return db;
		});
	}
	return _dbPromise;
}

// ─── Identity Key ─────────────────────────────────────────────────────────────

export async function storeIdentityKeyPair(kp: IdentityKeyPair): Promise<void> {
	const db = await getDB();
	await db.put('identity', kp, 'own');
}

export async function loadIdentityKeyPair(): Promise<IdentityKeyPair | null> {
	const db = await getDB();
	return (await db.get('identity', 'own')) ?? null;
}

// ─── One-Time PreKeys ─────────────────────────────────────────────────────────

export async function storePreKey(prekey: PreKeyPair): Promise<void> {
	const db = await getDB();
	await db.put('prekeys', prekey);
}

export async function loadPreKey(keyId: number): Promise<PreKeyPair | null> {
	const db = await getDB();
	return (await db.get('prekeys', keyId)) ?? null;
}

export async function deletePreKey(keyId: number): Promise<void> {
	const db = await getDB();
	await db.delete('prekeys', keyId);
}

export async function countPreKeys(): Promise<number> {
	const db = await getDB();
	return db.count('prekeys');
}

// ─── Signed PreKey ────────────────────────────────────────────────────────────

export async function storeSignedPreKey(spk: SignedPreKey): Promise<void> {
	const db = await getDB();
	await db.put('signed_prekeys', spk);
}

export async function loadSignedPreKey(keyId: number): Promise<SignedPreKey | null> {
	const db = await getDB();
	return (await db.get('signed_prekeys', keyId)) ?? null;
}

export async function loadLatestSignedPreKey(): Promise<SignedPreKey | null> {
	const db = await getDB();
	const all: SignedPreKey[] = await db.getAll('signed_prekeys');
	if (!all.length) return null;
	return all.reduce((a, b) => (a.createdAt > b.createdAt ? a : b));
}

// ─── Sessions ─────────────────────────────────────────────────────────────────

export async function storeSession(session: Session): Promise<void> {
	const db = await getDB();
	await db.put('sessions', session);
}

export async function loadSession(conversationId: string): Promise<Session | null> {
	const db = await getDB();
	return (await db.get('sessions', conversationId)) ?? null;
}

export async function deleteSession(conversationId: string): Promise<void> {
	const db = await getDB();
	await db.delete('sessions', conversationId);
}

// ─── Sender Keys (our own, per channel) ──────────────────────────────────────

export async function storeSenderKey(key: SenderKey): Promise<void> {
	const db = await getDB();
	await db.put('sender_keys', key);
}

export async function loadSenderKey(channelId: string): Promise<SenderKey | null> {
	const db = await getDB();
	return (await db.get('sender_keys', channelId)) ?? null;
}

export async function deleteSenderKey(channelId: string): Promise<void> {
	const db = await getDB();
	await db.delete('sender_keys', channelId);
}

// ─── Receiver Keys (one per sender per channel) ───────────────────────────────

function receiverKeyId(channelId: string, senderId: string): string {
	return `${channelId}:${senderId}`;
}

export async function storeReceiverKey(record: SenderKeyRecord): Promise<void> {
	const db = await getDB();
	await db.put('receiver_keys', record, receiverKeyId(record.channelId, record.senderId));
}

export async function loadReceiverKey(channelId: string, senderId: string): Promise<SenderKeyRecord | null> {
	const db = await getDB();
	return (await db.get('receiver_keys', receiverKeyId(channelId, senderId))) ?? null;
}

export async function deleteReceiverKey(channelId: string, senderId: string): Promise<void> {
	const db = await getDB();
	await db.delete('receiver_keys', receiverKeyId(channelId, senderId));
}
