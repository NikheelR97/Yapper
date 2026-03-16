import "fake-indexeddb/auto";

import { openDB } from "idb";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("$lib/desktop/vault.js", () => ({
  clearDesktopSignalVaultRecord: vi.fn(),
  desktopVaultSupported: vi.fn(() => false),
  loadDesktopSignalVaultRecord: vi.fn(async () => null),
  saveDesktopSignalVaultRecord: vi.fn(async () => {}),
}));

vi.mock("./idbCrypto.js", () => ({
  initIdbEncryption: vi.fn(async () => {}),
  clearIdbEncryptionKey: vi.fn(),
  isIdbEncryptionReady: vi.fn(() => true),
  idbEncryptValue: vi.fn(async (value: unknown) => value),
  idbDecryptValue: vi.fn(async <T>(value: unknown) => value as T),
}));

import * as idbCrypto from "./idbCrypto.js";
import {
  configureSignalStore,
  currentSignalDbName,
  getCachedEmojis,
  loadIdentityKeyPair,
  loadLatestSignedPreKey,
  loadPeerFingerprint,
  loadPeerTrustFlags,
  loadPreKey,
  loadReceiverKey,
  loadSenderKey,
  loadSignalBootstrapComplete,
  listSessionsForPeer,
  resetSignalStoreScope,
  storePeerFingerprint,
  storePeerKeyChanged,
  storePeerVerified,
} from "./keystore.js";

const LEGACY_DB_NAME = "yapper-signal";
const SCOPED_DB_NAME = "yapper-signal-user-1:device-1";

async function deleteDb(name: string): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const request = indexedDB.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
    request.onblocked = () => reject(new Error(`delete blocked for ${name}`));
  });
}

async function openSignalDbForTest(name: string) {
  return openDB(name, 6, {
    upgrade(db, oldVersion) {
      if (!db.objectStoreNames.contains("identity"))
        db.createObjectStore("identity");
      if (!db.objectStoreNames.contains("prekeys"))
        db.createObjectStore("prekeys", { keyPath: "keyId" });
      if (!db.objectStoreNames.contains("signed_prekeys")) {
        db.createObjectStore("signed_prekeys", { keyPath: "keyId" });
      }
      if (!db.objectStoreNames.contains("sessions")) {
        db.createObjectStore("sessions", { keyPath: "conversationId" });
      }
      if (oldVersion < 2) {
        db.createObjectStore("sender_keys", { keyPath: "channelId" });
        db.createObjectStore("receiver_keys");
      }
      if (oldVersion < 3) {
        db.createObjectStore("emojis");
      }
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
    },
  });
}

describe("keystore legacy migration", () => {
  beforeEach(async () => {
    resetSignalStoreScope();
    await deleteDb(LEGACY_DB_NAME);
    await deleteDb(SCOPED_DB_NAME);
  });

  afterEach(async () => {
    resetSignalStoreScope();
    await deleteDb(LEGACY_DB_NAME);
    await deleteDb(SCOPED_DB_NAME);
  });

  it("migrates legacy receiver keys and emoji cache into the scoped store", async () => {
    const legacyDb = await openSignalDbForTest(LEGACY_DB_NAME);
    await legacyDb.put(
      "identity",
      {
        dhPublicKey: new Uint8Array([1]),
        dhPrivateKey: new Uint8Array([2]),
        sigPublicKey: new Uint8Array([3]),
        sigPrivateKey: new Uint8Array([4]),
      },
      "own",
    );
    await legacyDb.put("prekeys", {
      keyId: 7,
      publicKey: new Uint8Array([5]),
      privateKey: new Uint8Array([6]),
    });
    await legacyDb.put("signed_prekeys", {
      keyId: 9,
      publicKey: new Uint8Array([7]),
      privateKey: new Uint8Array([8]),
      signature: new Uint8Array([9]),
      createdAt: Date.parse("2026-03-07T12:01:00.000Z"),
    });
    await legacyDb.put("sessions", {
      conversationId: "conv-1",
      peerId: "peer-1",
      rootKey: new Uint8Array([10]),
      sendChainKey: new Uint8Array([11]),
      receiveChainKey: new Uint8Array([12]),
      sendMsgNum: 1,
      receiveMsgNum: 0,
    });
    await legacyDb.put("sender_keys", {
      channelId: "channel-1",
      chainKey: new Uint8Array([13]),
      signingPubKey: new Uint8Array([14]),
      signingPrivKey: new Uint8Array([15]),
      iteration: 2,
    });
    await legacyDb.put(
      "receiver_keys",
      {
        channelId: "channel-1",
        senderId: "peer-1",
        senderDeviceId: "legacy",
        chainKey: new Uint8Array([16]),
        signingPubKey: new Uint8Array([17]),
        iteration: 1,
      },
      "channel-1:peer-1:legacy",
    );
    await legacyDb.put(
      "emojis",
      [
        {
          id: "emoji-1",
          name: "party",
          imageUrl: "https://cdn.test/party.png",
        },
      ],
      "server-1",
    );
    legacyDb.close();

    await expect(
      configureSignalStore("user-1", "device-1"),
    ).resolves.toBeUndefined();
    expect(currentSignalDbName()).toBe(SCOPED_DB_NAME);

    const identity = await loadIdentityKeyPair();
    expect(identity).not.toBeNull();
    expect(identity && Array.from(identity.dhPublicKey)).toEqual([1]);
    expect(identity && Array.from(identity.dhPrivateKey)).toEqual([2]);
    expect(await loadPreKey(7)).toMatchObject({ keyId: 7 });
    expect(await loadLatestSignedPreKey()).toMatchObject({ keyId: 9 });
    expect(await listSessionsForPeer("conv-1", "peer-1")).toHaveLength(1);
    expect(await loadSenderKey("channel-1")).toMatchObject({
      channelId: "channel-1",
      iteration: 2,
    });
    expect(
      await loadReceiverKey("channel-1", "peer-1", "legacy"),
    ).toMatchObject({
      channelId: "channel-1",
      senderId: "peer-1",
      iteration: 1,
    });
    expect(await getCachedEmojis("server-1")).toEqual([
      { id: "emoji-1", name: "party", imageUrl: "https://cdn.test/party.png" },
    ]);
    expect(await loadSignalBootstrapComplete()).toBe(true);
  });

  it("stores peer trust state in the scoped signal store", async () => {
    await configureSignalStore("user-1", "device-1");

    await storePeerFingerprint(
      "peer-1",
      "device-2",
      "12345 12345 12345 12345 12345 12345",
    );
    await storePeerKeyChanged("peer-1", true);

    expect(await loadPeerFingerprint("peer-1", "device-2")).toBe(
      "12345 12345 12345 12345 12345 12345",
    );
    expect(await loadPeerTrustFlags("peer-1")).toEqual({
      verified: false,
      keyChanged: true,
    });

    await storePeerVerified("peer-1", true);

    expect(await loadPeerTrustFlags("peer-1")).toEqual({
      verified: true,
      keyChanged: false,
    });
  });

  it("fails closed when IndexedDB encryption cannot initialize", async () => {
    vi.mocked(idbCrypto.initIdbEncryption).mockRejectedValueOnce(
      new Error("crypto unavailable"),
    );

    await expect(configureSignalStore("user-1", "device-1")).rejects.toThrow(
      /crypto unavailable/i,
    );
  });
});
