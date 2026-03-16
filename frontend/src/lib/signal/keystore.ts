/**
 * Persistent key storage using IndexedDB (via `idb`).
 *
 * Works in all WebView environments (Tauri, Capacitor, Web PWA).
 * The active store is namespaced by authenticated user/device so logout/login
 * does not silently rotate into another account's keystore.
 *
 * Store layout:
 *   identity       — singleton IdentityKeyPair (keyed by 'own')
 *   prekeys        — Map<keyId, PreKeyPair>
 *   signed_prekeys — Map<keyId, SignedPreKey>
 *   sessions       — Map<conversationId, Session>
 *   sender_keys    — Map<channelId, SenderKey>        (v2, our own key per channel)
 *   receiver_keys  — Map<channelId:senderId, SenderKeyRecord>  (v2, received keys)
 *   emojis         — Map<serverId, ServerEmoji[]>     (v3, custom emoji cache)
 */

import { openDB, type IDBPDatabase } from "idb";
import {
  clearDesktopSignalVaultRecord,
  desktopVaultSupported,
  loadDesktopSignalVaultRecord,
  saveDesktopSignalVaultRecord,
} from "$lib/desktop/vault.js";
import type {
  IdentityKeyPair,
  PreKeyPair,
  SignedPreKey,
  Session,
  SenderKey,
  SenderKeyRecord,
} from "./types.js";
import {
  initIdbEncryption,
  clearIdbEncryptionKey,
  isIdbEncryptionReady,
  idbEncryptValue,
  idbDecryptValue,
} from "./idbCrypto.js";

const DB_NAME = "yapper-signal";
const DB_NAME_PREFIX = "yapper-signal";
const DB_VERSION = 7;
const BOOTSTRAP_COMPLETE_KEY = "bootstrap-complete";

interface SignalSecretSnapshot {
  identityKey: IdentityKeyPair | null;
  prekeys: PreKeyPair[];
  signedPrekeys: SignedPreKey[];
  sessions: Session[];
  senderKeys: SenderKey[];
  receiverKeys: SenderKeyRecord[];
  bootstrapComplete: boolean;
}

export interface CachedDmHistoryMessage {
  id: string;
  conversation_id: string;
  sender_id: string;
  sender_device_id: string;
  sender_signal_device_id: number;
  ciphertext: string;
  ephemeral_key: string | null;
  opk_id: number | null;
  msg_num: number;
  ratchet_pub: string | null;
  previous_chain_len: number | null;
  crypto_version: number | null;
  created_at: string;
}

export interface CachedChannelHistoryMessage {
  id: string;
  channel_id: string;
  sender_id: string;
  sender_device_id: string | null;
  ciphertext: string | null;
  plaintext: string | null;
  message_type: string;
  msg_num: number | null;
  created_at: string;
}

let _db: IDBPDatabase | null = null;
let _dbPromise: Promise<IDBPDatabase> | null = null;
let _scopeKey: string | null = null;
let _desktopSecretSnapshot: SignalSecretSnapshot | null = null;

async function getDB(): Promise<IDBPDatabase> {
  if (_db) return _db;
  if (!_dbPromise) {
    _dbPromise = openSignalDb(currentDbName()).then((db) => {
      _db = db;
      return db;
    });
  }
  return _dbPromise;
}

async function openSignalDb(name: string): Promise<IDBPDatabase> {
  return openDB(name, DB_VERSION, {
    upgrade(db, oldVersion) {
      // v1 stores (always create if missing on fresh install)
      if (!db.objectStoreNames.contains("identity"))
        db.createObjectStore("identity");
      if (!db.objectStoreNames.contains("prekeys"))
        db.createObjectStore("prekeys", { keyPath: "keyId" });
      if (!db.objectStoreNames.contains("signed_prekeys"))
        db.createObjectStore("signed_prekeys", { keyPath: "keyId" });
      if (!db.objectStoreNames.contains("sessions"))
        db.createObjectStore("sessions", { keyPath: "conversationId" });
      // v2 — Sender Keys for group E2EE
      if (oldVersion < 2) {
        db.createObjectStore("sender_keys", { keyPath: "channelId" });
        db.createObjectStore("receiver_keys"); // keyed externally as `${channelId}:${senderId}`
      }
      // v3 — Custom emoji cache (keyed by serverId)
      if (oldVersion < 3) {
        db.createObjectStore("emojis");
      }
      // v4 - Signal bootstrap metadata
      if (oldVersion < 4) {
        db.createObjectStore("meta");
      }
      if (oldVersion < 5) {
        db.createObjectStore("dm_sessions", { keyPath: "sessionId" });
      }
      if (!db.objectStoreNames.contains("dm_history")) {
        const dmHistory = db.createObjectStore("dm_history", { keyPath: "id" });
        dmHistory.createIndex("by_conversation_created_at", [
          "conversation_id",
          "created_at",
        ]);
      }
      if (!db.objectStoreNames.contains("channel_history")) {
        const channelHistory = db.createObjectStore("channel_history", {
          keyPath: "id",
        });
        channelHistory.createIndex("by_channel_created_at", [
          "channel_id",
          "created_at",
        ]);
      }
      if (!db.objectStoreNames.contains("peer_trust")) {
        db.createObjectStore("peer_trust");
      }
    },
  });
}

export async function configureSignalStore(
  userId: string,
  deviceId: string,
): Promise<void> {
  const nextScope = `${userId}:${deviceId}`;
  if (_scopeKey === nextScope) return;

  _scopeKey = nextScope;
  _desktopSecretSnapshot = null;
  _db?.close();
  _db = null;
  _dbPromise = null;
  if (!useDesktopVault()) {
    await initIdbEncryption(nextScope);
    if (!isIdbEncryptionReady()) {
      throw new Error("IndexedDB encryption did not initialize");
    }
  }
  await getDB();
  await migrateLegacyStoreIfNeeded();
  await migrateSessionStoreIfNeeded();
  await migrateDesktopVaultIfNeeded();
}

export function resetSignalStoreScope(): void {
  _scopeKey = null;
  _desktopSecretSnapshot = null;
  clearIdbEncryptionKey();
  _db?.close();
  _db = null;
  _dbPromise = null;
}

export function currentSignalDbName(): string {
  return currentDbName();
}

function currentDbName(): string {
  return _scopeKey ? `${DB_NAME_PREFIX}-${_scopeKey}` : DB_NAME;
}

function useDesktopVault(): boolean {
  return desktopVaultSupported();
}

// ─── IndexedDB Encryption (M-05) ─────────────────────────────────────────────

const SENSITIVE_STORES = new Set([
  "identity",
  "prekeys",
  "signed_prekeys",
  "dm_sessions",
  "sender_keys",
  "receiver_keys",
]);

const STORE_KEY_PATHS: Record<string, string | undefined> = {
  prekeys: "keyId",
  signed_prekeys: "keyId",
  dm_sessions: "sessionId",
  sender_keys: "channelId",
};

function shouldEncryptIdb(): boolean {
  return !useDesktopVault() && isIdbEncryptionReady();
}

async function secPut(
  db: IDBPDatabase,
  storeName: string,
  value: unknown,
  key?: IDBValidKey,
): Promise<IDBValidKey> {
  if (shouldEncryptIdb() && SENSITIVE_STORES.has(storeName)) {
    const encrypted = await idbEncryptValue(value, STORE_KEY_PATHS[storeName]);
    return db.put(storeName, encrypted, key);
  }
  return db.put(storeName, value, key);
}

async function secGet<T>(
  db: IDBPDatabase,
  storeName: string,
  key: IDBValidKey,
): Promise<T | undefined> {
  const raw = await db.get(storeName, key);
  if (raw === undefined) return undefined;
  if (SENSITIVE_STORES.has(storeName)) {
    return idbDecryptValue<T>(raw);
  }
  return raw as T;
}

async function secGetAll<T>(db: IDBPDatabase, storeName: string): Promise<T[]> {
  const rawItems = await db.getAll(storeName);
  if (!SENSITIVE_STORES.has(storeName) || !rawItems.length) {
    return rawItems as T[];
  }
  return Promise.all(rawItems.map((item) => idbDecryptValue<T>(item)));
}

async function encryptForTx(
  storeName: string,
  value: unknown,
): Promise<unknown> {
  if (shouldEncryptIdb() && SENSITIVE_STORES.has(storeName)) {
    return idbEncryptValue(value, STORE_KEY_PATHS[storeName]);
  }
  return value;
}

function emptySignalSecretSnapshot(): SignalSecretSnapshot {
  return {
    identityKey: null,
    prekeys: [],
    signedPrekeys: [],
    sessions: [],
    senderKeys: [],
    receiverKeys: [],
    bootstrapComplete: false,
  };
}

function cloneSecretSnapshot(
  snapshot: SignalSecretSnapshot,
): SignalSecretSnapshot {
  return {
    identityKey: snapshot.identityKey,
    prekeys: [...snapshot.prekeys],
    signedPrekeys: [...snapshot.signedPrekeys],
    sessions: [...snapshot.sessions],
    senderKeys: [...snapshot.senderKeys],
    receiverKeys: [...snapshot.receiverKeys],
    bootstrapComplete: snapshot.bootstrapComplete,
  };
}

async function requireDesktopSecretSnapshot(): Promise<SignalSecretSnapshot> {
  if (!_scopeKey) {
    throw new Error("Signal store scope is not configured");
  }

  if (_desktopSecretSnapshot) {
    return _desktopSecretSnapshot;
  }

  _desktopSecretSnapshot =
    (await loadDesktopSignalVaultRecord<SignalSecretSnapshot>(_scopeKey)) ??
    emptySignalSecretSnapshot();
  return _desktopSecretSnapshot;
}

async function persistDesktopSecretSnapshot(
  snapshot: SignalSecretSnapshot,
): Promise<void> {
  if (!_scopeKey) {
    throw new Error("Signal store scope is not configured");
  }

  _desktopSecretSnapshot = cloneSecretSnapshot(snapshot);
  await saveDesktopSignalVaultRecord(_scopeKey, _desktopSecretSnapshot);
}

function mergeByKey<T>(
  existing: T[],
  incoming: T[],
  key: (item: T) => string | number,
): T[] {
  const next = new Map(existing.map((item) => [key(item), item]));
  for (const item of incoming) {
    next.set(key(item), item);
  }
  return Array.from(next.values());
}

async function listReceiverKeysFromDb(
  db: IDBPDatabase,
): Promise<SenderKeyRecord[]> {
  const receiverKeyIds = await db.getAllKeys("receiver_keys").catch(() => []);
  const receiverKeys: SenderKeyRecord[] = [];
  for (const key of receiverKeyIds) {
    const record = await secGet<SenderKeyRecord>(db, "receiver_keys", key);
    if (record) {
      receiverKeys.push(record);
    }
  }
  return receiverKeys;
}

async function readKeyedStoreEntries<T>(
  db: IDBPDatabase,
  storeName: string,
): Promise<Array<{ key: IDBValidKey; value: T }>> {
  const keys = await db.getAllKeys(storeName).catch(() => []);
  const values = await Promise.all(keys.map((key) => db.get(storeName, key)));
  const entries: Array<{ key: IDBValidKey; value: T }> = [];
  for (let index = 0; index < keys.length; index += 1) {
    const value = values[index];
    if (value !== undefined) {
      entries.push({ key: keys[index], value: value as T });
    }
  }
  return entries;
}

async function readSecretSnapshotFromDb(
  db: IDBPDatabase,
): Promise<SignalSecretSnapshot> {
  return {
    identityKey: (await secGet<IdentityKeyPair>(db, "identity", "own")) ?? null,
    prekeys: await secGetAll<PreKeyPair>(db, "prekeys"),
    signedPrekeys: await secGetAll<SignedPreKey>(db, "signed_prekeys"),
    sessions: await secGetAll<Session>(db, "dm_sessions"),
    senderKeys: await secGetAll<SenderKey>(db, "sender_keys"),
    receiverKeys: await listReceiverKeysFromDb(db),
    bootstrapComplete: (await db.get("meta", BOOTSTRAP_COMPLETE_KEY)) === true,
  };
}

async function clearIndexedDbSecretStores(db: IDBPDatabase): Promise<void> {
  const tx = db.transaction(
    [
      "identity",
      "prekeys",
      "signed_prekeys",
      "sessions",
      "dm_sessions",
      "sender_keys",
      "receiver_keys",
      "meta",
    ],
    "readwrite",
  );
  await tx.objectStore("identity").clear();
  await tx.objectStore("prekeys").clear();
  await tx.objectStore("signed_prekeys").clear();
  await tx.objectStore("sessions").clear();
  await tx.objectStore("dm_sessions").clear();
  await tx.objectStore("sender_keys").clear();
  await tx.objectStore("receiver_keys").clear();
  await tx.objectStore("meta").clear();
  await tx.done;
}

async function clearIndexedDbSecretStoresByName(name: string): Promise<void> {
  const db = await openSignalDb(name);
  try {
    await clearIndexedDbSecretStores(db);
  } finally {
    db.close();
  }
}

async function migrateLegacyStoreIfNeeded(): Promise<void> {
  if (!_scopeKey) return;
  if (currentDbName() === DB_NAME) return;

  const currentDb = await getDB();
  const identity = await currentDb.get("identity", "own");
  if (identity) return;

  const legacyDb = await openSignalDb(DB_NAME);
  const legacyIdentity = await legacyDb.get("identity", "own");
  if (!legacyIdentity) {
    legacyDb.close();
    return;
  }

  const [prekeys, signedPrekeys, sessions, dmSessions, senderKeys] =
    await Promise.all([
      legacyDb.getAll("prekeys"),
      legacyDb.getAll("signed_prekeys"),
      legacyDb.getAll("sessions"),
      legacyDb.getAll("dm_sessions").catch(() => []),
      legacyDb.getAll("sender_keys").catch(() => []),
    ]);
  const [receiverKeys, emojis] = await Promise.all([
    readKeyedStoreEntries<SenderKeyRecord>(legacyDb, "receiver_keys"),
    readKeyedStoreEntries(legacyDb, "emojis"),
  ]);

  // Pre-encrypt sensitive values before starting transaction (M-05)
  const encIdentity = await encryptForTx("identity", legacyIdentity);
  const encPrekeys = await Promise.all(
    prekeys.map((r) => encryptForTx("prekeys", r)),
  );
  const encSignedPrekeys = await Promise.all(
    signedPrekeys.map((r) => encryptForTx("signed_prekeys", r)),
  );
  const encDmSessions = await Promise.all(
    dmSessions.map((r) => encryptForTx("dm_sessions", r)),
  );
  const encMigratedSessions = await Promise.all(
    sessions.map((r) =>
      encryptForTx("dm_sessions", {
        ...r,
        sessionId: r.sessionId ?? `legacy:${r.conversationId}:${r.peerId}`,
        peerDeviceId: r.peerDeviceId ?? "legacy",
        peerSignalDeviceId: r.peerSignalDeviceId ?? 1,
      }),
    ),
  );
  const encSenderKeys = await Promise.all(
    senderKeys.map((r) => encryptForTx("sender_keys", r)),
  );
  const encReceiverKeys = await Promise.all(
    receiverKeys.map(async ({ key, value }) => ({
      key,
      value: await encryptForTx("receiver_keys", value),
    })),
  );

  const tx = currentDb.transaction(
    [
      "identity",
      "prekeys",
      "signed_prekeys",
      "sessions",
      "dm_sessions",
      "sender_keys",
      "receiver_keys",
      "emojis",
      "meta",
    ],
    "readwrite",
  );

  await tx.objectStore("identity").put(encIdentity, "own");
  for (const record of encPrekeys) await tx.objectStore("prekeys").put(record);
  for (const record of encSignedPrekeys)
    await tx.objectStore("signed_prekeys").put(record);
  for (const record of sessions) await tx.objectStore("sessions").put(record);
  for (const record of encDmSessions)
    await tx.objectStore("dm_sessions").put(record);
  for (const record of encMigratedSessions) {
    await tx.objectStore("dm_sessions").put(record);
  }
  for (const record of encSenderKeys)
    await tx.objectStore("sender_keys").put(record);
  for (const { key, value } of encReceiverKeys) {
    await tx.objectStore("receiver_keys").put(value, key);
  }
  for (const { key, value } of emojis) {
    await tx.objectStore("emojis").put(value, key);
  }
  await tx.objectStore("meta").put(true, BOOTSTRAP_COMPLETE_KEY);
  await tx.done;
  legacyDb.close();
}

async function migrateSessionStoreIfNeeded(): Promise<void> {
  const db = await getDB();
  const existingSessions = await db.getAll("dm_sessions");
  if (existingSessions.length > 0) {
    return;
  }

  const legacySessions = await db.getAll("sessions");
  if (!legacySessions.length) {
    return;
  }

  // Pre-encrypt before transaction (M-05)
  const encSessions = await Promise.all(
    legacySessions.map((record) =>
      encryptForTx("dm_sessions", {
        ...record,
        sessionId:
          record.sessionId ??
          `legacy:${record.conversationId}:${record.peerId}`,
        peerDeviceId: record.peerDeviceId ?? "legacy",
        peerSignalDeviceId: record.peerSignalDeviceId ?? 1,
      }),
    ),
  );
  const tx = db.transaction(["dm_sessions"], "readwrite");
  for (const record of encSessions) {
    await tx.objectStore("dm_sessions").put(record);
  }
  await tx.done;
}

async function migrateDesktopVaultIfNeeded(): Promise<void> {
  if (!useDesktopVault() || !_scopeKey) {
    return;
  }

  const persisted =
    await loadDesktopSignalVaultRecord<SignalSecretSnapshot>(_scopeKey);
  if (persisted) {
    _desktopSecretSnapshot = persisted;
    return;
  }

  const db = await getDB();
  const snapshot = await readSecretSnapshotFromDb(db);
  const hasSecrets =
    snapshot.identityKey != null ||
    snapshot.prekeys.length > 0 ||
    snapshot.signedPrekeys.length > 0 ||
    snapshot.sessions.length > 0 ||
    snapshot.senderKeys.length > 0 ||
    snapshot.receiverKeys.length > 0 ||
    snapshot.bootstrapComplete;

  _desktopSecretSnapshot = snapshot;
  if (hasSecrets) {
    await persistDesktopSecretSnapshot(snapshot);
    await clearIndexedDbSecretStores(db);
    if (currentDbName() !== DB_NAME) {
      await clearIndexedDbSecretStoresByName(DB_NAME).catch(() => {});
    }
  }
}

// ─── Identity Key ─────────────────────────────────────────────────────────────

export async function storeIdentityKeyPair(kp: IdentityKeyPair): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      identityKey: kp,
    });
    return;
  }

  const db = await getDB();
  await secPut(db, "identity", kp, "own");
}

export async function loadIdentityKeyPair(): Promise<IdentityKeyPair | null> {
  if (useDesktopVault()) {
    return (await requireDesktopSecretSnapshot()).identityKey;
  }

  const db = await getDB();
  return (await secGet<IdentityKeyPair>(db, "identity", "own")) ?? null;
}

// ─── One-Time PreKeys ─────────────────────────────────────────────────────────

export async function storePreKey(prekey: PreKeyPair): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      prekeys: mergeByKey(snapshot.prekeys, [prekey], (item) => item.keyId),
    });
    return;
  }

  const db = await getDB();
  await secPut(db, "prekeys", prekey);
}

export async function loadPreKey(keyId: number): Promise<PreKeyPair | null> {
  if (useDesktopVault()) {
    return (
      (await requireDesktopSecretSnapshot()).prekeys.find(
        (record) => record.keyId === keyId,
      ) ?? null
    );
  }

  const db = await getDB();
  return (await secGet<PreKeyPair>(db, "prekeys", keyId)) ?? null;
}

export async function deletePreKey(keyId: number): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      prekeys: snapshot.prekeys.filter((record) => record.keyId !== keyId),
    });
    return;
  }

  const db = await getDB();
  await db.delete("prekeys", keyId);
}

export async function countPreKeys(): Promise<number> {
  if (useDesktopVault()) {
    return (await requireDesktopSecretSnapshot()).prekeys.length;
  }

  const db = await getDB();
  return db.count("prekeys");
}

export async function listPreKeys(): Promise<PreKeyPair[]> {
  if (useDesktopVault()) {
    return [...(await requireDesktopSecretSnapshot()).prekeys];
  }

  const db = await getDB();
  return secGetAll<PreKeyPair>(db, "prekeys");
}

// ─── Signed PreKey ────────────────────────────────────────────────────────────

export async function storeSignedPreKey(spk: SignedPreKey): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      signedPrekeys: mergeByKey(
        snapshot.signedPrekeys,
        [spk],
        (item) => item.keyId,
      ),
    });
    return;
  }

  const db = await getDB();
  await secPut(db, "signed_prekeys", spk);
}

export async function loadSignedPreKey(
  keyId: number,
): Promise<SignedPreKey | null> {
  if (useDesktopVault()) {
    return (
      (await requireDesktopSecretSnapshot()).signedPrekeys.find(
        (record) => record.keyId === keyId,
      ) ?? null
    );
  }

  const db = await getDB();
  return (await secGet<SignedPreKey>(db, "signed_prekeys", keyId)) ?? null;
}

export async function loadLatestSignedPreKey(): Promise<SignedPreKey | null> {
  if (useDesktopVault()) {
    const all = (await requireDesktopSecretSnapshot()).signedPrekeys;
    if (!all.length) return null;
    return all.reduce((a, b) => (a.createdAt > b.createdAt ? a : b));
  }

  const db = await getDB();
  const all: SignedPreKey[] = await secGetAll<SignedPreKey>(
    db,
    "signed_prekeys",
  );
  if (!all.length) return null;
  return all.reduce((a, b) => (a.createdAt > b.createdAt ? a : b));
}

// ─── Sessions ─────────────────────────────────────────────────────────────────

export async function storeSession(session: Session): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      sessions: mergeByKey(
        snapshot.sessions,
        [session],
        (item) => item.sessionId,
      ),
    });
    return;
  }

  const db = await getDB();
  await secPut(db, "dm_sessions", session);
}

export async function loadSession(sessionId: string): Promise<Session | null> {
  if (useDesktopVault()) {
    return (
      (await requireDesktopSecretSnapshot()).sessions.find(
        (record) => record.sessionId === sessionId,
      ) ?? null
    );
  }

  const db = await getDB();
  return (await secGet<Session>(db, "dm_sessions", sessionId)) ?? null;
}

export async function listSessionsForPeer(
  conversationId: string,
  peerId: string,
): Promise<Session[]> {
  if (useDesktopVault()) {
    return (await requireDesktopSecretSnapshot()).sessions.filter(
      (session) =>
        session.conversationId === conversationId && session.peerId === peerId,
    );
  }

  const db = await getDB();
  const sessions = await secGetAll<Session>(db, "dm_sessions");
  return sessions.filter(
    (session) =>
      session.conversationId === conversationId && session.peerId === peerId,
  );
}

export async function deleteSession(sessionId: string): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      sessions: snapshot.sessions.filter(
        (record) => record.sessionId !== sessionId,
      ),
    });
    return;
  }

  const db = await getDB();
  await db.delete("dm_sessions", sessionId);
}

// ─── Sender Keys (our own, per channel) ──────────────────────────────────────

export async function storeSenderKey(key: SenderKey): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      senderKeys: mergeByKey(
        snapshot.senderKeys,
        [key],
        (item) => item.channelId,
      ),
    });
    return;
  }

  const db = await getDB();
  await secPut(db, "sender_keys", key);
}

export async function loadSenderKey(
  channelId: string,
): Promise<SenderKey | null> {
  if (useDesktopVault()) {
    return (
      (await requireDesktopSecretSnapshot()).senderKeys.find(
        (record) => record.channelId === channelId,
      ) ?? null
    );
  }

  const db = await getDB();
  return (await secGet<SenderKey>(db, "sender_keys", channelId)) ?? null;
}

export async function deleteSenderKey(channelId: string): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      senderKeys: snapshot.senderKeys.filter(
        (record) => record.channelId !== channelId,
      ),
    });
    return;
  }

  const db = await getDB();
  await db.delete("sender_keys", channelId);
}

// ─── Receiver Keys (one per sender per channel) ───────────────────────────────

function receiverKeyId(
  channelId: string,
  senderId: string,
  senderDeviceId = "legacy",
): string {
  return `${channelId}:${senderId}:${senderDeviceId}`;
}

export async function storeReceiverKey(record: SenderKeyRecord): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      receiverKeys: mergeByKey(snapshot.receiverKeys, [record], (item) =>
        receiverKeyId(item.channelId, item.senderId, item.senderDeviceId),
      ),
    });
    return;
  }

  const db = await getDB();
  await secPut(
    db,
    "receiver_keys",
    record,
    receiverKeyId(record.channelId, record.senderId, record.senderDeviceId),
  );
}

export async function loadReceiverKey(
  channelId: string,
  senderId: string,
  senderDeviceId = "legacy",
): Promise<SenderKeyRecord | null> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    const exact =
      snapshot.receiverKeys.find(
        (record) =>
          record.channelId === channelId &&
          record.senderId === senderId &&
          record.senderDeviceId === senderDeviceId,
      ) ?? null;
    if (exact) return exact;
    if (senderDeviceId !== "legacy") {
      return (
        snapshot.receiverKeys.find(
          (record) =>
            record.channelId === channelId &&
            record.senderId === senderId &&
            record.senderDeviceId === "legacy",
        ) ?? null
      );
    }
    return null;
  }

  const db = await getDB();
  const exact =
    (await secGet<SenderKeyRecord>(
      db,
      "receiver_keys",
      receiverKeyId(channelId, senderId, senderDeviceId),
    )) ?? null;
  if (exact) return exact;
  if (senderDeviceId !== "legacy") {
    return (
      (await secGet<SenderKeyRecord>(
        db,
        "receiver_keys",
        receiverKeyId(channelId, senderId, "legacy"),
      )) ?? null
    );
  }
  return null;
}

export async function deleteReceiverKey(
  channelId: string,
  senderId: string,
  senderDeviceId = "legacy",
): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      receiverKeys: snapshot.receiverKeys.filter(
        (record) =>
          !(
            record.channelId === channelId &&
            record.senderId === senderId &&
            record.senderDeviceId === senderDeviceId
          ),
      ),
    });
    return;
  }

  const db = await getDB();
  await db.delete(
    "receiver_keys",
    receiverKeyId(channelId, senderId, senderDeviceId),
  );
}

// ─── Emoji Cache (v3) ─────────────────────────────────────────────────────────

export interface CachedEmoji {
  id: string;
  name: string;
  imageUrl: string;
}

export async function getCachedEmojis(
  serverId: string,
): Promise<CachedEmoji[] | null> {
  const db = await getDB();
  return (await db.get("emojis", serverId)) ?? null;
}

export async function setCachedEmojis(
  serverId: string,
  emojis: CachedEmoji[],
): Promise<void> {
  const db = await getDB();
  await db.put("emojis", emojis, serverId);
}

export async function storeDmHistoryMessages(
  messages: CachedDmHistoryMessage[],
): Promise<void> {
  if (!messages.length) return;
  const db = await getDB();
  const tx = db.transaction(["dm_history"], "readwrite");
  for (const message of messages) {
    await tx.objectStore("dm_history").put(message);
  }
  await tx.done;
}

export async function listDmHistoryMessages(
  conversationId: string,
): Promise<CachedDmHistoryMessage[]> {
  const db = await getDB();
  return db.getAllFromIndex(
    "dm_history",
    "by_conversation_created_at",
    IDBKeyRange.bound([conversationId, ""], [conversationId, "\uffff"]),
  );
}

export async function storeChannelHistoryMessages(
  messages: CachedChannelHistoryMessage[],
): Promise<void> {
  if (!messages.length) return;
  const db = await getDB();
  const tx = db.transaction(["channel_history"], "readwrite");
  for (const message of messages) {
    await tx.objectStore("channel_history").put(message);
  }
  await tx.done;
}

export async function listChannelHistoryMessages(
  channelId: string,
): Promise<CachedChannelHistoryMessage[]> {
  const db = await getDB();
  return db.getAllFromIndex(
    "channel_history",
    "by_channel_created_at",
    IDBKeyRange.bound([channelId, ""], [channelId, "\uffff"]),
  );
}

export async function loadSignalBootstrapComplete(): Promise<boolean> {
  if (useDesktopVault()) {
    return (await requireDesktopSecretSnapshot()).bootstrapComplete;
  }

  const db = await getDB();
  return (await db.get("meta", BOOTSTRAP_COMPLETE_KEY)) === true;
}

export async function storeSignalBootstrapComplete(
  value: boolean,
): Promise<void> {
  if (useDesktopVault()) {
    const snapshot = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      ...snapshot,
      bootstrapComplete: value,
    });
    return;
  }

  const db = await getDB();
  await db.put("meta", value, BOOTSTRAP_COMPLETE_KEY);
}

function peerFingerprintKey(peerId: string, peerDeviceId: string): string {
  return `fingerprint:${peerId}:${peerDeviceId}`;
}

function peerVerifiedKey(peerId: string): string {
  return `verified:${peerId}`;
}

function peerKeyChangedKey(peerId: string): string {
  return `changed:${peerId}`;
}

export async function loadPeerFingerprint(
  peerId: string,
  peerDeviceId: string,
): Promise<string | null> {
  const db = await getDB();
  return (
    (await db.get("peer_trust", peerFingerprintKey(peerId, peerDeviceId))) ??
    null
  );
}

export async function storePeerFingerprint(
  peerId: string,
  peerDeviceId: string,
  fingerprint: string,
): Promise<void> {
  const db = await getDB();
  await db.put(
    "peer_trust",
    fingerprint,
    peerFingerprintKey(peerId, peerDeviceId),
  );
}

export async function loadPeerTrustFlags(
  peerId: string,
): Promise<{ verified: boolean; keyChanged: boolean }> {
  const db = await getDB();
  const [verified, keyChanged] = await Promise.all([
    db.get("peer_trust", peerVerifiedKey(peerId)),
    db.get("peer_trust", peerKeyChangedKey(peerId)),
  ]);
  return {
    verified: verified === true,
    keyChanged: keyChanged === true,
  };
}

export async function storePeerVerified(
  peerId: string,
  verified: boolean,
): Promise<void> {
  const db = await getDB();
  if (verified) {
    await db.put("peer_trust", true, peerVerifiedKey(peerId));
    await db.delete("peer_trust", peerKeyChangedKey(peerId));
    return;
  }
  await db.delete("peer_trust", peerVerifiedKey(peerId));
}

export async function storePeerKeyChanged(
  peerId: string,
  keyChanged: boolean,
): Promise<void> {
  const db = await getDB();
  if (keyChanged) {
    await db.put("peer_trust", true, peerKeyChangedKey(peerId));
    return;
  }
  await db.delete("peer_trust", peerKeyChangedKey(peerId));
}

export async function exportSignalSnapshot(): Promise<{
  identityKey: IdentityKeyPair | null;
  prekeys: PreKeyPair[];
  signedPrekeys: SignedPreKey[];
  sessions: Session[];
  senderKeys: SenderKey[];
  receiverKeys: SenderKeyRecord[];
  dmHistory: CachedDmHistoryMessage[];
  channelHistory: CachedChannelHistoryMessage[];
  bootstrapComplete: boolean;
}> {
  if (useDesktopVault()) {
    const db = await getDB();
    const snapshot = await requireDesktopSecretSnapshot();
    return {
      ...cloneSecretSnapshot(snapshot),
      dmHistory: await db.getAll("dm_history"),
      channelHistory: await db.getAll("channel_history"),
    };
  }

  const db = await getDB();
  const receiverKeys = await listReceiverKeysFromDb(db);

  return {
    identityKey: (await db.get("identity", "own")) ?? null,
    prekeys: await db.getAll("prekeys"),
    signedPrekeys: await db.getAll("signed_prekeys"),
    sessions: await db.getAll("dm_sessions"),
    senderKeys: await db.getAll("sender_keys"),
    receiverKeys,
    dmHistory: await db.getAll("dm_history"),
    channelHistory: await db.getAll("channel_history"),
    bootstrapComplete: (await db.get("meta", BOOTSTRAP_COMPLETE_KEY)) === true,
  };
}

export async function importSignalSnapshot(snapshot: {
  identityKey?: IdentityKeyPair | null;
  prekeys?: PreKeyPair[];
  signedPrekeys?: SignedPreKey[];
  sessions?: Session[];
  senderKeys?: SenderKey[];
  receiverKeys?: SenderKeyRecord[];
  dmHistory?: CachedDmHistoryMessage[];
  channelHistory?: CachedChannelHistoryMessage[];
  bootstrapComplete?: boolean;
}): Promise<void> {
  if (useDesktopVault()) {
    const current = await requireDesktopSecretSnapshot();
    await persistDesktopSecretSnapshot({
      identityKey:
        snapshot.identityKey !== undefined
          ? snapshot.identityKey
          : current.identityKey,
      prekeys: mergeByKey(
        current.prekeys,
        snapshot.prekeys ?? [],
        (item) => item.keyId,
      ),
      signedPrekeys: mergeByKey(
        current.signedPrekeys,
        snapshot.signedPrekeys ?? [],
        (item) => item.keyId,
      ),
      sessions: mergeByKey(
        current.sessions,
        snapshot.sessions ?? [],
        (item) => item.sessionId,
      ),
      senderKeys: mergeByKey(
        current.senderKeys,
        snapshot.senderKeys ?? [],
        (item) => item.channelId,
      ),
      receiverKeys: mergeByKey(
        current.receiverKeys,
        snapshot.receiverKeys ?? [],
        (item) =>
          receiverKeyId(item.channelId, item.senderId, item.senderDeviceId),
      ),
      bootstrapComplete:
        snapshot.bootstrapComplete != null
          ? snapshot.bootstrapComplete
          : current.bootstrapComplete,
    });

    const db = await getDB();
    const tx = db.transaction(["dm_history", "channel_history"], "readwrite");
    for (const dmMessage of snapshot.dmHistory ?? []) {
      await tx.objectStore("dm_history").put(dmMessage);
    }
    for (const channelMessage of snapshot.channelHistory ?? []) {
      await tx.objectStore("channel_history").put(channelMessage);
    }
    await tx.done;
    return;
  }

  const db = await getDB();

  // Pre-encrypt sensitive values before starting transaction (M-05)
  const encIdentity = snapshot.identityKey
    ? await encryptForTx("identity", snapshot.identityKey)
    : null;
  const encPrekeys = await Promise.all(
    (snapshot.prekeys ?? []).map((pk) => encryptForTx("prekeys", pk)),
  );
  const encSignedPrekeys = await Promise.all(
    (snapshot.signedPrekeys ?? []).map((spk) =>
      encryptForTx("signed_prekeys", spk),
    ),
  );
  const encSessions = await Promise.all(
    (snapshot.sessions ?? []).map((s) => encryptForTx("dm_sessions", s)),
  );
  const encSenderKeys = await Promise.all(
    (snapshot.senderKeys ?? []).map((sk) => encryptForTx("sender_keys", sk)),
  );
  const srcReceiverKeys = snapshot.receiverKeys ?? [];
  const encReceiverKeys = await Promise.all(
    srcReceiverKeys.map((rk) => encryptForTx("receiver_keys", rk)),
  );

  const tx = db.transaction(
    [
      "identity",
      "prekeys",
      "signed_prekeys",
      "dm_sessions",
      "sender_keys",
      "receiver_keys",
      "dm_history",
      "channel_history",
      "meta",
    ],
    "readwrite",
  );

  if (encIdentity) {
    await tx.objectStore("identity").put(encIdentity, "own");
  }
  for (const prekey of encPrekeys) {
    await tx.objectStore("prekeys").put(prekey);
  }
  for (const signedPreKey of encSignedPrekeys) {
    await tx.objectStore("signed_prekeys").put(signedPreKey);
  }
  for (const session of encSessions) {
    await tx.objectStore("dm_sessions").put(session);
  }
  for (const senderKey of encSenderKeys) {
    await tx.objectStore("sender_keys").put(senderKey);
  }
  for (let i = 0; i < encReceiverKeys.length; i++) {
    const rk = srcReceiverKeys[i];
    await tx
      .objectStore("receiver_keys")
      .put(
        encReceiverKeys[i],
        receiverKeyId(rk.channelId, rk.senderId, rk.senderDeviceId),
      );
  }
  for (const dmMessage of snapshot.dmHistory ?? []) {
    await tx.objectStore("dm_history").put(dmMessage);
  }
  for (const channelMessage of snapshot.channelHistory ?? []) {
    await tx.objectStore("channel_history").put(channelMessage);
  }
  if (snapshot.bootstrapComplete != null) {
    await tx
      .objectStore("meta")
      .put(snapshot.bootstrapComplete, BOOTSTRAP_COMPLETE_KEY);
  }

  await tx.done;
}

export async function clearCurrentSignalStore(): Promise<void> {
  const dbName = currentDbName();
  if (useDesktopVault() && _scopeKey) {
    await clearDesktopSignalVaultRecord(_scopeKey);
    _desktopSecretSnapshot = null;
  }
  clearIdbEncryptionKey(true);

  _db?.close();
  _db = null;
  _dbPromise = null;
  await new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase(dbName);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
    request.onblocked = () =>
      reject(new Error("Signal store deletion blocked"));
  });
}
