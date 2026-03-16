/**
 * IndexedDB encryption at rest (M-05).
 *
 * Encrypts sensitive Signal Protocol key material in IndexedDB using
 * AES-256-GCM. The encryption key is kept non-exportable and persisted
 * in a dedicated IndexedDB key store. Desktop platforms use Stronghold
 * vault instead — this module is unused there.
 */

import { openDB, type IDBPDatabase } from "idb";

let _cachedKey: CryptoKey | null = null;
let _scopeKey: string | null = null;
let _keyDb: IDBPDatabase | null = null;
let _keyDbPromise: Promise<IDBPDatabase> | null = null;

const KEY_DB_NAME = "yapper-signal-crypto";
const KEY_STORE_NAME = "keys";
const KEY_DB_VERSION = 1;

function storageKeyName(scope: string): string {
  return `yapper-enc-${scope}`;
}

async function getKeyDb(): Promise<IDBPDatabase> {
  if (_keyDb) return _keyDb;
  if (!_keyDbPromise) {
    _keyDbPromise = openDB(KEY_DB_NAME, KEY_DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(KEY_STORE_NAME)) {
          db.createObjectStore(KEY_STORE_NAME);
        }
      },
    }).then((db) => {
      _keyDb = db;
      return db;
    });
  }
  return _keyDbPromise;
}

async function loadStoredKey(scope: string): Promise<CryptoKey | null> {
  const db = await getKeyDb();
  return (await db.get(KEY_STORE_NAME, storageKeyName(scope))) ?? null;
}

async function storeStoredKey(scope: string, key: CryptoKey): Promise<void> {
  const db = await getKeyDb();
  await db.put(KEY_STORE_NAME, key, storageKeyName(scope));
}

async function removeStoredKey(scope: string): Promise<void> {
  const db = await getKeyDb();
  await db.delete(KEY_STORE_NAME, storageKeyName(scope));
}

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function b64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function jsonReplacer(_key: string, value: unknown): unknown {
  if (value instanceof Uint8Array) {
    return { __t: "u8", __d: bytesToB64(value) };
  }
  return value;
}

function jsonReviver(_key: string, value: unknown): unknown {
  if (
    value &&
    typeof value === "object" &&
    (value as Record<string, unknown>).__t === "u8" &&
    typeof (value as Record<string, unknown>).__d === "string"
  ) {
    return b64ToBytes((value as Record<string, string>).__d);
  }
  return value;
}

export interface EncryptedEnvelope {
  __yenc: 1;
  iv: string;
  ct: string;
  [key: string]: unknown;
}

function isEncryptedEnvelope(value: unknown): value is EncryptedEnvelope {
  return (
    value !== null &&
    typeof value === "object" &&
    (value as Record<string, unknown>).__yenc === 1
  );
}

async function getOrCreateKey(): Promise<CryptoKey> {
  if (_cachedKey) return _cachedKey;
  if (!_scopeKey) {
    throw new Error("IDB encryption not initialized");
  }

  const persisted = await loadStoredKey(_scopeKey);
  if (persisted) {
    _cachedKey = persisted;
    return _cachedKey;
  }

  const legacyStored = localStorage.getItem(storageKeyName(_scopeKey));
  if (legacyStored) {
    const raw = b64ToBytes(legacyStored);
    _cachedKey = await crypto.subtle.importKey(
      "raw",
      raw.slice(),
      "AES-GCM",
      false,
      ["encrypt", "decrypt"],
    );
    await storeStoredKey(_scopeKey, _cachedKey);
    localStorage.removeItem(storageKeyName(_scopeKey));
    return _cachedKey;
  }

  _cachedKey = await crypto.subtle.generateKey(
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
  await storeStoredKey(_scopeKey, _cachedKey);
  return _cachedKey;
}

export async function initIdbEncryption(scopeKey: string): Promise<void> {
  if (typeof indexedDB === "undefined") {
    throw new Error("IndexedDB is unavailable for secure key storage");
  }
  if (typeof crypto?.subtle === "undefined") {
    throw new Error("WebCrypto is unavailable for secure key storage");
  }

  _scopeKey = scopeKey;
  _cachedKey = null;
  await getOrCreateKey();
}

export function clearIdbEncryptionKey(removeFromStorage = false): void {
  if (removeFromStorage && _scopeKey) {
    const scope = _scopeKey;
    void removeStoredKey(scope).catch(() => {});
    try {
      localStorage.removeItem(storageKeyName(scope));
    } catch {
      // ignore
    }
  }
  _cachedKey = null;
  _scopeKey = null;
}

export function isIdbEncryptionReady(): boolean {
  return _cachedKey !== null && _scopeKey !== null;
}

/**
 * Encrypt a value for IndexedDB storage.
 * If keyPathField is provided, its value is preserved unencrypted so
 * IndexedDB can still use it as the object store key.
 */
export async function idbEncryptValue(
  value: unknown,
  keyPathField?: string,
): Promise<EncryptedEnvelope> {
  const key = await getOrCreateKey();
  const plaintext = new TextEncoder().encode(
    JSON.stringify(value, jsonReplacer),
  );
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ct = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv },
    key,
    plaintext,
  );

  const envelope: EncryptedEnvelope = {
    __yenc: 1,
    iv: bytesToB64(iv),
    ct: bytesToB64(new Uint8Array(ct)),
  };

  if (keyPathField && value && typeof value === "object") {
    envelope[keyPathField] = (value as Record<string, unknown>)[keyPathField];
  }

  return envelope;
}

/**
 * Decrypt a value from IndexedDB.
 * Returns the value as-is if it's not an encrypted envelope (backwards compat).
 */
export async function idbDecryptValue<T>(value: unknown): Promise<T> {
  if (!isEncryptedEnvelope(value)) {
    return value as T;
  }

  const key = await getOrCreateKey();
  const iv = b64ToBytes(value.iv);
  const ct = b64ToBytes(value.ct);
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: iv.slice() },
    key,
    ct.slice(),
  );

  return JSON.parse(new TextDecoder().decode(plaintext), jsonReviver) as T;
}
