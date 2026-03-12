import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { describe, expect, it } from "vitest";
import {
  decryptSenderKeyDist,
  decryptWithSenderKey,
  encryptSenderKeyDist,
  encryptWithSenderKey,
  generateSenderKey,
  signSenderKeyDistPayload,
  verifySenderKeyDistPayload,
} from "./sender_keys.js";
import type { IdentityKeyPair, SenderKeyRecord } from "./types.js";

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

describe("sender_keys historical decrypt", () => {
  it("allows historical decrypt without rewinding live ratchet state", async () => {
    const channelId = "channel-1";
    const senderId = "user-2";
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();

    let senderKey = generateSenderKey(channelId);
    const seedChainKey = senderKey.chainKey.slice();
    const signingPubKey = senderKey.signingPubKey.slice();

    const encrypted = [];
    for (const text of ["one", "two", "three"]) {
      const out = await encryptWithSenderKey(senderKey, encoder.encode(text));
      encrypted.push(out.encrypted);
      senderKey = out.updatedKey;
    }

    let record: SenderKeyRecord = {
      channelId,
      senderId,
      senderDeviceId: "device-2",
      chainKey: seedChainKey,
      signingPubKey,
      iteration: 0,
      initialChainKey: seedChainKey,
      initialIteration: 0,
    };

    for (const [idx, expected] of ["one", "two", "three"].entries()) {
      const out = await decryptWithSenderKey(record, encrypted[idx]);
      expect(decoder.decode(out.plaintext)).toBe(expected);
      record = out.updatedRecord;
    }

    expect(record.iteration).toBe(3);

    await expect(decryptWithSenderKey(record, encrypted[0])).rejects.toThrow(
      /already consumed/i,
    );

    const historical = await decryptWithSenderKey(record, encrypted[0], {
      allowHistorical: true,
    });
    expect(decoder.decode(historical.plaintext)).toBe("one");
    expect(historical.updatedRecord.iteration).toBe(3);
  });

  it("rejects forged sender-key distributions after decrypt", async () => {
    const senderDhPrivateKey = x25519.utils.randomSecretKey();
    const senderSigPrivateKey = ed25519.utils.randomSecretKey();
    const senderIdentity: IdentityKeyPair = {
      dhPublicKey: x25519.getPublicKey(senderDhPrivateKey),
      dhPrivateKey: senderDhPrivateKey,
      sigPublicKey: new Uint8Array(ed25519.getPublicKey(senderSigPrivateKey)),
      sigPrivateKey: senderSigPrivateKey,
    };

    const recipientPrivateKey = x25519.utils.randomSecretKey();
    const recipientPublicKey = x25519.getPublicKey(recipientPrivateKey);

    const payload = signSenderKeyDistPayload(
      {
        channelId: "channel-1",
        senderUserId: "sender-1",
        senderDeviceId: "device-1",
        chainKey: btoa("a".repeat(32)),
        signingPubKey: btoa("b".repeat(32)),
        iteration: 0,
      },
      senderIdentity,
    );

    const encrypted = await encryptSenderKeyDist(
      payload,
      recipientPublicKey,
      senderIdentity.dhPublicKey,
    );
    const decrypted = await decryptSenderKeyDist(
      encrypted.ciphertext,
      encrypted.ephemeralKey,
      recipientPrivateKey,
      recipientPublicKey,
      senderIdentity.dhPublicKey,
    );

    expect(() =>
      verifySenderKeyDistPayload(
        decrypted,
        senderIdentity.sigPublicKey,
        "sender-1",
        "device-1",
      ),
    ).not.toThrow();

    expect(() =>
      verifySenderKeyDistPayload(
        { ...decrypted, channelId: "channel-2" },
        senderIdentity.sigPublicKey,
        "sender-1",
        "device-1",
      ),
    ).toThrow(/identity signature/i);
  });
});

describe("ECIES IKM binding — senderDhPub required in IKM", () => {
  it("round-trips a sender key distribution when senderDhPub matches", async () => {
    const senderIdentity = makeIdentity();
    const recipientDhPriv = x25519.utils.randomSecretKey();
    const recipientDhPub = x25519.getPublicKey(recipientDhPriv);

    const senderKey = generateSenderKey("channel-ikm-1");
    const payload = signSenderKeyDistPayload(
      {
        channelId: "channel-ikm-1",
        senderUserId: "user-a",
        senderDeviceId: "device-a",
        chainKey: bytesToB64(senderKey.chainKey),
        signingPubKey: bytesToB64(senderKey.signingPubKey),
        iteration: 0,
      },
      senderIdentity,
    );

    const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(
      payload,
      recipientDhPub,
      senderIdentity.dhPublicKey,
    );

    const decrypted = await decryptSenderKeyDist(
      ciphertext,
      ephemeralKey,
      recipientDhPriv,
      recipientDhPub,
      senderIdentity.dhPublicKey,
    );

    expect(decrypted.channelId).toBe("channel-ikm-1");
    expect(decrypted.senderUserId).toBe("user-a");
    expect(decrypted.chainKey).toBe(bytesToB64(senderKey.chainKey));
    expect(decrypted.signingPubKey).toBe(bytesToB64(senderKey.signingPubKey));
  });

  it("fails to decrypt when a wrong senderDhPub is provided (IKM mismatch → AES-GCM auth failure)", async () => {
    // This test documents WHY the 3-component IKM fallback was removed.
    // The old code allowed decryptSenderKeyDist to be called without senderDhPub,
    // which produced a different IKM than encryption always used, causing AES-GCM
    // authentication tag failures and silently dropping key distributions.
    const senderIdentity = makeIdentity();
    const impostorIdentity = makeIdentity(); // different keys — server-substitution scenario
    const recipientDhPriv = x25519.utils.randomSecretKey();
    const recipientDhPub = x25519.getPublicKey(recipientDhPriv);

    const senderKey = generateSenderKey("channel-ikm-2");
    const payload = signSenderKeyDistPayload(
      {
        channelId: "channel-ikm-2",
        senderUserId: "user-a",
        senderDeviceId: "device-a",
        chainKey: bytesToB64(senderKey.chainKey),
        signingPubKey: bytesToB64(senderKey.signingPubKey),
        iteration: 0,
      },
      senderIdentity,
    );

    const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(
      payload,
      recipientDhPub,
      senderIdentity.dhPublicKey, // encrypted binding: real sender
    );

    // Decrypting with a different senderDhPub → wrong IKM → AES-GCM throws
    await expect(
      decryptSenderKeyDist(
        ciphertext,
        ephemeralKey,
        recipientDhPriv,
        recipientDhPub,
        impostorIdentity.dhPublicKey, // wrong — IKM won't match
      ),
    ).rejects.toThrow();
  });

  it("full sender key encrypt-distribute-decrypt-message round-trip across two identities", async () => {
    const senderIdentity = makeIdentity();
    const recipientDhPriv = x25519.utils.randomSecretKey();
    const recipientDhPub = x25519.getPublicKey(recipientDhPriv);

    // Sender generates and distributes their key
    let senderKey = generateSenderKey("channel-ikm-3");
    const payload = signSenderKeyDistPayload(
      {
        channelId: "channel-ikm-3",
        senderUserId: "user-a",
        senderDeviceId: "device-a",
        chainKey: bytesToB64(senderKey.chainKey),
        signingPubKey: bytesToB64(senderKey.signingPubKey),
        iteration: 0,
      },
      senderIdentity,
    );

    const { ciphertext: distCt, ephemeralKey } = await encryptSenderKeyDist(
      payload,
      recipientDhPub,
      senderIdentity.dhPublicKey,
    );

    // Recipient decrypts the distribution
    const decryptedPayload = await decryptSenderKeyDist(
      distCt,
      ephemeralKey,
      recipientDhPriv,
      recipientDhPub,
      senderIdentity.dhPublicKey,
    );

    const receiverRecord: SenderKeyRecord = {
      channelId: decryptedPayload.channelId,
      senderId: "user-a",
      senderDeviceId: "device-a",
      chainKey: Uint8Array.from(atob(decryptedPayload.chainKey), (c) => c.charCodeAt(0)),
      signingPubKey: Uint8Array.from(
        atob(decryptedPayload.signingPubKey),
        (c) => c.charCodeAt(0),
      ),
      iteration: decryptedPayload.iteration,
      initialChainKey: Uint8Array.from(
        atob(decryptedPayload.chainKey),
        (c) => c.charCodeAt(0),
      ),
      initialIteration: decryptedPayload.iteration,
    };

    // Sender encrypts a message
    const encoder = new TextEncoder();
    const { encrypted, updatedKey } = await encryptWithSenderKey(
      senderKey,
      encoder.encode("hello from channel"),
    );
    senderKey = updatedKey;

    // Recipient decrypts it using the stored record
    const { plaintext } = await decryptWithSenderKey(receiverRecord, encrypted);
    expect(new TextDecoder().decode(plaintext)).toBe("hello from channel");
  });
});
