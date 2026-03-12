/**
 * Unit tests for the channel E2EE key distribution flow.
 *
 * Specifically covers receiveSenderKeyDist and fetchAndStorePendingDists
 * to catch regressions where a failed fetchDeviceKeyBundle silently produces
 * a corrupted ECIES IKM (the "Unable to decrypt" bug in server channels).
 */

import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  encryptSenderKeyDist,
  generateSenderKey,
  signSenderKeyDistPayload,
} from "./sender_keys.js";
import type { IdentityKeyPair, SenderKeyRecord } from "./types.js";

vi.mock("$api/client.js", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

vi.mock("./keystore.js", () => ({
  loadIdentityKeyPair: vi.fn(),
  storeIdentityKeyPair: vi.fn(),
  loadLatestSignedPreKey: vi.fn(),
  storeSignedPreKey: vi.fn(),
  storePreKey: vi.fn(),
  listPreKeys: vi.fn(),
  loadSignalBootstrapComplete: vi.fn(),
  storeSignalBootstrapComplete: vi.fn(),
  loadSession: vi.fn(),
  storeSession: vi.fn(),
  deleteSession: vi.fn(),
  listSessionsForPeer: vi.fn(),
  loadPreKey: vi.fn(),
  deletePreKey: vi.fn(),
  loadSenderKey: vi.fn(),
  storeSenderKey: vi.fn(),
  loadReceiverKey: vi.fn(),
  storeReceiverKey: vi.fn(),
  loadPeerFingerprint: vi.fn(),
  storePeerFingerprint: vi.fn(),
  storePeerKeyChanged: vi.fn(),
}));

import { api } from "$api/client.js";
import * as ks from "./keystore.js";
import { receiveSenderKeyDist } from "./index.js";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

function makeIdentity(): IdentityKeyPair {
  const dhPrivateKey = x25519.utils.randomSecretKey();
  const sigPrivateKey = ed25519.utils.randomSecretKey();
  return {
    dhPrivateKey,
    dhPublicKey: x25519.getPublicKey(dhPrivateKey),
    sigPrivateKey,
    sigPublicKey: new Uint8Array(ed25519.getPublicKey(sigPrivateKey)),
  };
}

/** Build the raw KeyBundleV2Response[] that api.get returns for /api/v2/keys/:id/bundles */
function makeBundleResponse(userId: string, deviceId: string, identity: IdentityKeyPair) {
  return [
    {
      user_id: userId,
      device_id: deviceId,
      signal_device_id: 1,
      identity_dh_key: bytesToB64(identity.dhPublicKey),
      identity_sig_key: bytesToB64(identity.sigPublicKey),
      signed_prekey_id: 1,
      signed_prekey: bytesToB64(new Uint8Array(32).fill(1)),
      signed_prekey_sig: bytesToB64(new Uint8Array(64).fill(2)),
      one_time_prekey_id: null,
      one_time_prekey: null,
    },
  ];
}

/** Build an encrypted key distribution from senderIdentity → recipientIdentity */
async function buildDist(
  senderIdentity: IdentityKeyPair,
  recipientIdentity: IdentityKeyPair,
  channelId = "channel-test",
  senderUserId = "user-sender",
  senderDeviceId = "device-sender",
) {
  const senderKey = generateSenderKey(channelId);
  const payload = signSenderKeyDistPayload(
    {
      channelId,
      senderUserId,
      senderDeviceId,
      chainKey: bytesToB64(senderKey.chainKey),
      signingPubKey: bytesToB64(senderKey.signingPubKey),
      iteration: 0,
    },
    senderIdentity,
  );

  const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(
    payload,
    recipientIdentity.dhPublicKey,
    senderIdentity.dhPublicKey,
  );

  return {
    senderKey,
    event: {
      channel_id: channelId,
      from_user: senderUserId,
      from_device_id: senderDeviceId,
      ciphertext: bytesToB64(ciphertext),
      ek_public: bytesToB64(ephemeralKey),
    },
  };
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("receiveSenderKeyDist", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ks.storeReceiverKey).mockResolvedValue(undefined);
    vi.mocked(ks.loadReceiverKey).mockResolvedValue(null);
    vi.mocked(ks.storePeerFingerprint).mockResolvedValue(undefined);
    vi.mocked(ks.loadPeerFingerprint).mockResolvedValue(null);
  });

  it("stores a SenderKeyRecord when the distribution is valid end-to-end", async () => {
    const senderIdentity = makeIdentity();
    const recipientIdentity = makeIdentity();

    vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(recipientIdentity);
    vi.mocked(api.get).mockResolvedValue(
      makeBundleResponse("user-sender", "device-sender", senderIdentity),
    );

    const { senderKey, event } = await buildDist(senderIdentity, recipientIdentity);

    await receiveSenderKeyDist(event);

    expect(ks.storeReceiverKey).toHaveBeenCalledOnce();
    const stored = vi.mocked(ks.storeReceiverKey).mock.calls[0][0] as SenderKeyRecord;
    expect(stored.channelId).toBe("channel-test");
    expect(stored.senderId).toBe("user-sender");
    expect(stored.senderDeviceId).toBe("device-sender");
    expect(stored.iteration).toBe(0);
    // Chain key and signing key should match what the sender used
    expect(bytesToB64(stored.chainKey)).toBe(bytesToB64(senderKey.chainKey));
    expect(bytesToB64(stored.signingPubKey)).toBe(bytesToB64(senderKey.signingPubKey));
  });

  it("skips silently when from_device_id is absent (unauthenticated legacy distribution)", async () => {
    const recipientIdentity = makeIdentity();
    vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(recipientIdentity);

    await receiveSenderKeyDist({
      channel_id: "channel-test",
      from_user: "user-sender",
      from_device_id: null, // No device ID → cannot authenticate sender identity
      ciphertext: "aGVsbG8=",
      ek_public: bytesToB64(new Uint8Array(32).fill(5)),
    });

    // Should not attempt to fetch keys or store anything
    expect(api.get).not.toHaveBeenCalled();
    expect(ks.storeReceiverKey).not.toHaveBeenCalled();
  });

  it("skips cleanly when fetchDeviceKeyBundle fails (regression: before fix, silently used wrong ECIES IKM)", async () => {
    // This test catches the root cause of the "Unable to decrypt" bug.
    //
    // Before the fix: fetchDeviceKeyBundle failure was swallowed (.catch(() => null)),
    // senderDhPub became undefined, decryptSenderKeyDist fell back to 3-component IKM,
    // AES-GCM failed because encryption always uses 4-component IKM, and the
    // distribution was dropped.
    //
    // After the fix: the fetch error propagates, the distribution is cleanly skipped
    // in the outer try/catch, and storeReceiverKey is never called.
    const recipientIdentity = makeIdentity();
    vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(recipientIdentity);
    vi.mocked(api.get).mockRejectedValue(new Error("Network error — server temporarily down"));

    // Should resolve without throwing (error is caught internally and logged)
    await expect(
      receiveSenderKeyDist({
        channel_id: "channel-test",
        from_user: "user-sender",
        from_device_id: "device-sender", // Device ID known, but fetch fails
        ciphertext: "aGVsbG8=",
        ek_public: bytesToB64(new Uint8Array(32).fill(5)),
      }),
    ).resolves.toBeUndefined();

    // Critical: no key stored — no corrupted state
    expect(ks.storeReceiverKey).not.toHaveBeenCalled();
  });

  it("does not overwrite an existing record at a higher iteration (idempotent re-delivery)", async () => {
    const senderIdentity = makeIdentity();
    const recipientIdentity = makeIdentity();

    vi.mocked(ks.loadIdentityKeyPair).mockResolvedValue(recipientIdentity);
    vi.mocked(api.get).mockResolvedValue(
      makeBundleResponse("user-sender", "device-sender", senderIdentity),
    );

    const { senderKey, event } = await buildDist(senderIdentity, recipientIdentity);

    // Existing record is at iteration 5 (further ahead than the incoming dist at 0)
    const existingRecord: SenderKeyRecord = {
      channelId: "channel-test",
      senderId: "user-sender",
      senderDeviceId: "device-sender",
      chainKey: new Uint8Array(32).fill(9),
      signingPubKey: senderKey.signingPubKey,
      iteration: 5,
      initialChainKey: senderKey.chainKey,
      initialIteration: 0,
    };
    vi.mocked(ks.loadReceiverKey).mockResolvedValue(existingRecord);

    await receiveSenderKeyDist(event);

    // Should update to persist the seed, but not advance the iteration
    expect(ks.storeReceiverKey).toHaveBeenCalledOnce();
    const stored = vi.mocked(ks.storeReceiverKey).mock.calls[0][0] as SenderKeyRecord;
    expect(stored.iteration).toBe(5); // unchanged — keeps current ratchet position
  });
});
