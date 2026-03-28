import { x25519 } from "@noble/curves/ed25519.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { openDB, type IDBPDatabase } from "idb";
import { api } from "$api/client.js";
import {
  exportSignalSnapshot,
  importSignalSnapshot,
} from "$signal/keystore.js";
import type {
  CachedChannelHistoryMessage,
  CachedDmHistoryMessage,
} from "$signal/keystore.js";

const SYNC_DB_NAME = "yapper-device-sync";
const SYNC_DB_VERSION = 1;
const SYNC_STORE = "sync_requests";
const SYNC_INFO = new TextEncoder().encode("YapperDeviceSync_v1");
const HISTORY_PAGE_LIMIT = 100;

interface StoredSyncRequest {
  privateKey: string;
  publicKey: string;
}

interface SyncConversation {
  id: string;
}

interface SyncServer {
  id: string;
}

interface SyncChannel {
  id: string;
}

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return btoa(binary);
}

function b64ToBytes(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (value) => value.charCodeAt(0));
}

function concat(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((sum, array) => sum + array.length, 0);
  const output = new Uint8Array(total);
  let offset = 0;
  for (const array of arrays) {
    output.set(array, offset);
    offset += array.length;
  }
  return output;
}

// ── IndexedDB-backed sync request storage ────────────────────────────────────
// Private keys for pending device-sync requests are stored in IndexedDB rather
// than localStorage to reduce XSS exfiltration surface (localStorage is trivially
// enumerable; IndexedDB requires knowing the database and store names).

let _syncDb: IDBPDatabase | null = null;

async function getSyncDb(): Promise<IDBPDatabase> {
  if (_syncDb) return _syncDb;
  _syncDb = await openDB(SYNC_DB_NAME, SYNC_DB_VERSION, {
    upgrade(db) {
      if (!db.objectStoreNames.contains(SYNC_STORE)) {
        db.createObjectStore(SYNC_STORE);
      }
    },
  });
  return _syncDb;
}

async function loadStoredSyncRequest(
  deviceId: string,
): Promise<StoredSyncRequest | null> {
  if (typeof indexedDB === "undefined") return null;
  try {
    const db = await getSyncDb();
    const value = await db.get(SYNC_STORE, deviceId);
    if (value && value.privateKey && value.publicKey) {
      return value as StoredSyncRequest;
    }
    return null;
  } catch {
    /* IndexedDB unavailable (e.g., SSR or opaque origin) */
    return null;
  }
}

async function saveStoredSyncRequest(
  deviceId: string,
  request: StoredSyncRequest,
): Promise<void> {
  if (typeof indexedDB === "undefined") return;
  try {
    const db = await getSyncDb();
    await db.put(SYNC_STORE, request, deviceId);
  } catch {
    /* IndexedDB unavailable — key lives only in memory for this session */
  }
}

async function deleteStoredSyncRequest(deviceId: string): Promise<void> {
  if (typeof indexedDB === "undefined") return;
  try {
    const db = await getSyncDb();
    await db.delete(SYNC_STORE, deviceId);
  } catch {
    /* IndexedDB unavailable */
  }
}

function mergeById<T extends { id: string }>(
  primary: T[],
  secondary: T[],
): T[] {
  const records = new Map<string, T>();
  for (const item of primary) {
    records.set(item.id, item);
  }
  for (const item of secondary) {
    records.set(item.id, item);
  }
  return [...records.values()].sort((a, b) => {
    const aCreatedAt =
      "created_at" in a && typeof a.created_at === "string" ? a.created_at : "";
    const bCreatedAt =
      "created_at" in b && typeof b.created_at === "string" ? b.created_at : "";
    return Date.parse(aCreatedAt) - Date.parse(bCreatedAt);
  });
}

function jsonReplacer(_key: string, value: unknown): unknown {
  if (value instanceof Uint8Array) {
    return { __u8: bytesToB64(value) };
  }
  return value;
}

function jsonReviver(_key: string, value: unknown): unknown {
  if (value && typeof value === "object" && "__u8" in (value as object)) {
    return b64ToBytes((value as { __u8: string }).__u8);
  }
  return value;
}

/**
 * Create or retrieve a pending device-sync key pair for the given device.
 *
 * The X25519 private key is stored in IndexedDB (`yapper-device-sync`) rather
 * than localStorage to reduce XSS key exfiltration surface.
 *
 * @param deviceId — the local device UUID
 * @returns the base64-encoded X25519 public key for the sync handshake
 * @security Private key never leaves IndexedDB; cleared on sync completion or logout.
 */
export async function ensurePendingDeviceSyncRequest(deviceId: string): Promise<{
  publicKey: string;
}> {
  const existing = await loadStoredSyncRequest(deviceId);
  if (existing) {
    return { publicKey: existing.publicKey };
  }

  const privateKey = x25519.utils.randomSecretKey();
  const publicKey = x25519.getPublicKey(privateKey);
  const request = {
    privateKey: bytesToB64(privateKey),
    publicKey: bytesToB64(publicKey),
  };
  await saveStoredSyncRequest(deviceId, request);
  return { publicKey: request.publicKey };
}

/** Remove the pending sync request (and its private key) from IndexedDB. */
export async function clearPendingDeviceSyncRequest(deviceId: string): Promise<void> {
  await deleteStoredSyncRequest(deviceId);
}

/** Check whether a pending sync request exists for the given device. */
export async function hasPendingDeviceSyncRequest(deviceId: string): Promise<boolean> {
  return (await loadStoredSyncRequest(deviceId)) != null;
}

async function getPendingDeviceSyncPrivateKey(deviceId: string): Promise<Uint8Array> {
  const request = await loadStoredSyncRequest(deviceId);
  if (!request) {
    throw new Error("Missing pending device sync request key");
  }
  return b64ToBytes(request.privateKey);
}

/**
 * Encrypt a device-sync payload using ephemeral X25519 + HKDF + AES-256-GCM.
 *
 * @param plaintext — the serialised snapshot bytes to encrypt
 * @param recipientPublicKey — base64-encoded X25519 public key of the new device
 * @returns ciphertext (base64 of IV || AES-GCM output) and the ephemeral public key
 * @e2ee Ephemeral key is discarded after encryption — provides forward secrecy for the sync.
 */
export async function encryptDeviceSyncChunk(
  plaintext: Uint8Array,
  recipientPublicKey: string,
): Promise<{ ciphertext: string; ekPublic: string }> {
  const recipientPublicKeyBytes = b64ToBytes(recipientPublicKey);
  const ephemeralPrivateKey = x25519.utils.randomSecretKey();
  const ephemeralPublicKey = x25519.getPublicKey(ephemeralPrivateKey);
  const sharedSecret = x25519.getSharedSecret(
    ephemeralPrivateKey,
    recipientPublicKeyBytes,
  );
  const inputKeyMaterial = concat(
    sharedSecret,
    ephemeralPublicKey,
    recipientPublicKeyBytes,
  );
  const encryptionKey = hkdf(
    sha256,
    inputKeyMaterial,
    undefined,
    SYNC_INFO,
    32,
  );

  const iv = crypto.getRandomValues(new Uint8Array(12));
  const aesKey = await crypto.subtle.importKey(
    "raw",
    encryptionKey.slice(),
    "AES-GCM",
    false,
    ["encrypt"],
  );
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },
    aesKey,
    plaintext.slice(),
  );

  return {
    ciphertext: bytesToB64(concat(iv, new Uint8Array(ciphertext))),
    ekPublic: bytesToB64(ephemeralPublicKey),
  };
}

/**
 * Decrypt a device-sync chunk using the stored X25519 private key.
 *
 * @param deviceId — local device UUID (used to look up the private key in IndexedDB)
 * @param ciphertext — base64-encoded IV || AES-GCM ciphertext
 * @param ekPublic — base64-encoded ephemeral X25519 public key from the sender
 * @returns the decrypted plaintext bytes
 * @throws if no pending sync request exists or decryption fails
 */
export async function decryptDeviceSyncChunk(
  deviceId: string,
  ciphertext: string,
  ekPublic: string,
): Promise<Uint8Array> {
  const requestPrivateKey = await getPendingDeviceSyncPrivateKey(deviceId);
  const ephemeralPublicKey = b64ToBytes(ekPublic);
  const myPublicKey = x25519.getPublicKey(requestPrivateKey);
  const ciphertextBytes = b64ToBytes(ciphertext);
  const sharedSecret = x25519.getSharedSecret(
    requestPrivateKey,
    ephemeralPublicKey,
  );
  const inputKeyMaterial = concat(
    sharedSecret,
    ephemeralPublicKey,
    myPublicKey,
  );
  const encryptionKey = hkdf(
    sha256,
    inputKeyMaterial,
    undefined,
    SYNC_INFO,
    32,
  );

  const iv = ciphertextBytes.slice(0, 12);
  const encryptedBytes = ciphertextBytes.slice(12);
  const aesKey = await crypto.subtle.importKey(
    "raw",
    encryptionKey.slice(),
    "AES-GCM",
    false,
    ["decrypt"],
  );
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv },
    aesKey,
    encryptedBytes,
  );
  return new Uint8Array(plaintext);
}

async function fetchConversationHistory(): Promise<CachedDmHistoryMessage[]> {
  const conversations = await api
    .get<SyncConversation[]>("/api/v2/conversations")
    .catch(() => []);
  if (!conversations.length) {
    return [];
  }

  const histories = await Promise.all(
    conversations.map((conversation) =>
      fetchPaginatedHistory<CachedDmHistoryMessage>(
        `/api/v2/conversations/${conversation.id}/messages`,
      ),
    ),
  );
  return histories.flat();
}

async function fetchChannelHistory(): Promise<CachedChannelHistoryMessage[]> {
  const servers = await api
    .get<SyncServer[]>("/api/v2/servers")
    .catch(() => []);
  if (!servers.length) {
    return [];
  }

  const channels = (
    await Promise.all(
      servers.map((server) =>
        api
          .get<SyncChannel[]>(`/api/v2/servers/${server.id}/channels`)
          .catch(() => []),
      ),
    )
  ).flat();

  if (!channels.length) {
    return [];
  }

  const histories = await Promise.all(
    channels.map((channel) =>
      fetchPaginatedHistory<CachedChannelHistoryMessage>(
        `/api/v2/channels/${channel.id}/messages`,
      ),
    ),
  );
  return histories.flat();
}

async function fetchPaginatedHistory<T extends { id: string }>(
  basePath: string,
): Promise<T[]> {
  let before: string | null = null;
  let history: T[] = [];

  while (true) {
    const params = new URLSearchParams({
      limit: String(HISTORY_PAGE_LIMIT),
    });
    if (before) {
      params.set("before", before);
    }

    const page = await api
      .get<T[]>(`${basePath}?${params.toString()}`)
      .catch(() => []);
    if (!page.length) {
      break;
    }

    history = [...page, ...history];
    if (page.length < HISTORY_PAGE_LIMIT) {
      break;
    }

    const nextBefore = page[0]?.id ?? null;
    if (!nextBefore || nextBefore === before) {
      break;
    }
    before = nextBefore;
  }

  return history;
}

/**
 * Export a complete device snapshot (signal sessions, receiver keys, message history)
 * for transfer to a newly-trusted device.
 *
 * Merges remote server history with the local IndexedDB cache, preferring cached
 * ciphertext for messages that exist in both (the local copy has the correct
 * encryption context for the current device).
 *
 * @returns JSON-serialised snapshot string (Uint8Arrays encoded as `{ __u8: base64 }`)
 * @e2ee The snapshot contains ciphertext and signal session state — never plaintext.
 */
export async function exportTrustedDeviceSnapshot(): Promise<string> {
  const snapshot = await exportSignalSnapshot();
  const [remoteDmHistory, remoteChannelHistory] = await Promise.all([
    fetchConversationHistory(),
    fetchChannelHistory(),
  ]);
  return JSON.stringify(
    {
      sessions: snapshot.sessions,
      receiverKeys: snapshot.receiverKeys,
      dmHistory: mergeById(remoteDmHistory, snapshot.dmHistory),
      channelHistory: mergeById(remoteChannelHistory, snapshot.channelHistory),
    },
    jsonReplacer,
  );
}

/**
 * Import a decrypted device-sync snapshot into the local signal store.
 *
 * @param serializedSnapshot — JSON string from {@link exportTrustedDeviceSnapshot}
 * @e2ee Restores signal sessions, receiver keys, and message ciphertext history.
 */
export async function importTrustedDeviceSnapshot(
  serializedSnapshot: string,
): Promise<void> {
  const snapshot = JSON.parse(serializedSnapshot, jsonReviver) as {
    sessions?: Parameters<typeof importSignalSnapshot>[0]["sessions"];
    receiverKeys?: Parameters<typeof importSignalSnapshot>[0]["receiverKeys"];
    dmHistory?: Parameters<typeof importSignalSnapshot>[0]["dmHistory"];
    channelHistory?: Parameters<
      typeof importSignalSnapshot
    >[0]["channelHistory"];
  };
  await importSignalSnapshot({
    sessions: snapshot.sessions,
    receiverKeys: snapshot.receiverKeys,
    dmHistory: snapshot.dmHistory,
    channelHistory: snapshot.channelHistory,
  });
}
