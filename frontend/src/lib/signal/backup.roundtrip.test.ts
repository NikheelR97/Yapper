import "fake-indexeddb/auto";

import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/api/client.js", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
  },
}));

vi.mock("$lib/desktop/vault.js", () => ({
  clearDesktopSignalVaultRecord: vi.fn(),
  desktopVaultSupported: vi.fn(() => false),
  loadDesktopSignalVaultRecord: vi.fn(async () => null),
  saveDesktopSignalVaultRecord: vi.fn(async () => {}),
}));

import { api } from "$lib/api/client.js";
import { BACKUP_KDF_ITERATIONS, backupKeys, restoreKeys } from "./backup.js";
import {
  clearCurrentSignalStore,
  configureSignalStore,
  exportSignalSnapshot,
  importSignalSnapshot,
  loadIdentityKeyPair,
  loadPreKey,
  loadReceiverKey,
  loadSignalBootstrapComplete,
  loadSenderKey,
  loadSignedPreKey,
  listSessionsForPeer,
  resetSignalStoreScope,
  storeIdentityKeyPair,
  storePreKey,
  storeReceiverKey,
  storeSenderKey,
  storeSession,
  storeSignalBootstrapComplete,
  storeSignedPreKey,
} from "./keystore.js";
import type {
  IdentityKeyPair,
  PreKeyPair,
  SenderKey,
  SenderKeyRecord,
  Session,
  SignedPreKey,
} from "./types.js";

const PASS_PHRASE = "AlphaPass2468";
const BACKUP_BLOB_VERSION = 1;
const USER_ID = "user-1";
const DEVICE_ID = "device-1";
const DB_NAME = "yapper-signal";
const SCOPED_DB_NAME = `${DB_NAME}-${USER_ID}:${DEVICE_ID}`;

function bytes(fill: number, length: number): Uint8Array {
  return new Uint8Array(length).fill(fill);
}

function _u8ToB64(u8: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < u8.length; index += 1) {
    binary += String.fromCharCode(u8[index]);
  }
  return btoa(binary);
}

function b64ToU8(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (char) => char.charCodeAt(0));
}

async function deriveKey(
  passphrase: string,
  salt: Uint8Array,
): Promise<CryptoKey> {
  const keyMaterial = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(passphrase),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return crypto.subtle.deriveKey(
    {
      name: "PBKDF2",
      salt: salt.slice(),
      hash: "SHA-256",
      iterations: BACKUP_KDF_ITERATIONS,
    },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

async function decryptBackupBlob(
  passphrase: string,
  encryptedBlob: string,
): Promise<string> {
  const blob = b64ToU8(encryptedBlob);
  expect(blob[0]).toBe(BACKUP_BLOB_VERSION);

  const salt = blob.slice(1, 17);
  const iv = blob.slice(17, 29);
  const ciphertext = blob.slice(29);
  const key = await deriveKey(passphrase, salt);
  const decrypted = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: iv.slice() },
    key,
    ciphertext.slice(),
  );
  return new TextDecoder().decode(decrypted);
}

async function deleteDb(name: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error(`delete blocked for ${name}`));
  });
}

async function seedStore(): Promise<void> {
  await configureSignalStore(USER_ID, DEVICE_ID);

  const identity: IdentityKeyPair = {
    dhPublicKey: bytes(1, 32),
    dhPrivateKey: bytes(2, 32),
    sigPublicKey: bytes(3, 32),
    sigPrivateKey: bytes(4, 32),
  };
  const prekey: PreKeyPair = {
    keyId: 7,
    publicKey: bytes(5, 32),
    privateKey: bytes(6, 32),
  };
  const signedPrekey: SignedPreKey = {
    keyId: 9,
    publicKey: bytes(7, 32),
    privateKey: bytes(8, 32),
    signature: bytes(9, 64),
    createdAt: Date.parse("2026-03-26T12:00:00.000Z"),
  };
  const session: Session = {
    sessionId: "conv-1:peer-1:device-2",
    conversationId: "conv-1",
    peerId: "peer-1",
    peerDeviceId: "device-2",
    peerSignalDeviceId: 5,
    version: 2,
    rootKey: bytes(10, 32),
    sendChainKey: bytes(11, 32),
    receiveChainKey: bytes(12, 32),
    sendMsgNum: 1,
    receiveMsgNum: 0,
  };
  const senderKey: SenderKey = {
    channelId: "channel-1",
    chainKey: bytes(13, 32),
    signingPubKey: bytes(14, 32),
    signingPrivKey: bytes(15, 32),
    iteration: 2,
  };
  const receiverKey: SenderKeyRecord = {
    channelId: "channel-1",
    senderId: "peer-1",
    senderDeviceId: "device-2",
    chainKey: bytes(16, 32),
    signingPubKey: bytes(17, 32),
    iteration: 1,
  };

  await storeIdentityKeyPair(identity);
  await storePreKey(prekey);
  await storeSignedPreKey(signedPrekey);
  await storeSession(session);
  await storeSenderKey(senderKey);
  await storeReceiverKey(receiverKey);
  await storeSignalBootstrapComplete(true);
}

describe("signal backup round trip", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    resetSignalStoreScope();
    await deleteDb(DB_NAME);
    await deleteDb(SCOPED_DB_NAME);
  });

  it("exports canonical secrets and restores them without double-encrypting", async () => {
    await seedStore();

    await backupKeys(PASS_PHRASE);
    const putCall = vi.mocked(api.put).mock.calls[0];
    expect(putCall[0]).toBe("/api/v2/keys/backup");

    const uploaded = putCall[1] as { encrypted_blob: string };
    const snapshotJson = await decryptBackupBlob(
      PASS_PHRASE,
      uploaded.encrypted_blob,
    );
    expect(snapshotJson).toContain('"formatVersion":2');
    expect(snapshotJson).not.toContain("__yenc");
    expect(JSON.parse(snapshotJson)).toMatchObject({
      formatVersion: 2,
      bootstrapComplete: true,
    });

    await clearCurrentSignalStore();
    resetSignalStoreScope();
    await configureSignalStore(USER_ID, DEVICE_ID);

    vi.mocked(api.get).mockResolvedValue({
      encrypted_blob: uploaded.encrypted_blob,
    });

    await expect(restoreKeys(PASS_PHRASE)).resolves.toBe(true);

    const restored = await exportSignalSnapshot();
    expect(restored.formatVersion).toBe(2);
    expect(JSON.stringify(restored)).not.toContain("__yenc");
    expect(await loadIdentityKeyPair()).toMatchObject({
      dhPublicKey: bytes(1, 32),
      dhPrivateKey: bytes(2, 32),
      sigPublicKey: bytes(3, 32),
      sigPrivateKey: bytes(4, 32),
    });
    expect(await loadPreKey(7)).toMatchObject({ keyId: 7 });
    expect(await loadSignedPreKey(9)).toMatchObject({ keyId: 9 });
    expect(await listSessionsForPeer("conv-1", "peer-1")).toHaveLength(1);
    expect(await loadSenderKey("channel-1")).toMatchObject({
      channelId: "channel-1",
      iteration: 2,
    });
    expect(
      await loadReceiverKey("channel-1", "peer-1", "device-2"),
    ).toMatchObject({
      channelId: "channel-1",
      senderId: "peer-1",
      senderDeviceId: "device-2",
      iteration: 1,
    });
    expect(await loadSignalBootstrapComplete()).toBe(true);
  });

  it("rejects legacy envelope-shaped imports without a backup format version", async () => {
    await configureSignalStore(USER_ID, DEVICE_ID);

    await expect(
      importSignalSnapshot({
        identityKey: { __yenc: 1, iv: "iv", ct: "ct" } as never,
        prekeys: [],
        signedPrekeys: [],
        sessions: [],
        senderKeys: [],
        receiverKeys: [],
        bootstrapComplete: false,
        dmHistory: [],
        channelHistory: [],
      }),
    ).rejects.toThrow(/legacy signal backup envelope format/i);

    await clearCurrentSignalStore();
    resetSignalStoreScope();
  });
});
