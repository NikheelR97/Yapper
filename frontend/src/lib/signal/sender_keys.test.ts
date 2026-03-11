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

    const encrypted = await encryptSenderKeyDist(payload, recipientPublicKey);
    const decrypted = await decryptSenderKeyDist(
      encrypted.ciphertext,
      encrypted.ephemeralKey,
      recipientPrivateKey,
      recipientPublicKey,
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
