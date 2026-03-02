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
 */

import { openDB, type IDBPDatabase } from 'idb';
import type { IdentityKeyPair, PreKeyPair, SignedPreKey, Session } from './types.js';

const DB_NAME = 'yapper-signal';
const DB_VERSION = 1;

let _db: IDBPDatabase | null = null;

async function getDB(): Promise<IDBPDatabase> {
	if (_db) return _db;
	_db = await openDB(DB_NAME, DB_VERSION, {
		upgrade(db) {
			db.createObjectStore('identity');
			db.createObjectStore('prekeys', { keyPath: 'keyId' });
			db.createObjectStore('signed_prekeys', { keyPath: 'keyId' });
			db.createObjectStore('sessions', { keyPath: 'conversationId' });
		},
	});
	return _db;
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
