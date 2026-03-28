import { ed25519, x25519 } from "@noble/curves/ed25519.js";
import { describe, expect, it } from "vitest";
import { decryptRatchet, encryptRatchet } from "./ratchet.js";
import { x3dhInitiate, x3dhRespond } from "./x3dh.js";
import type {
  IdentityKeyPair,
  KeyBundle,
  Session,
  SignedPreKey,
} from "./types.js";

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return btoa(binary);
}

function newIdentity(): IdentityKeyPair {
  const dhPrivateKey = x25519.utils.randomSecretKey();
  const sigPrivateKey = ed25519.utils.randomSecretKey();
  return {
    dhPrivateKey,
    dhPublicKey: x25519.getPublicKey(dhPrivateKey),
    sigPrivateKey,
    sigPublicKey: ed25519.getPublicKey(sigPrivateKey),
  };
}

function signedPreKey(identity: IdentityKeyPair, keyId = 1): SignedPreKey {
  const privateKey = x25519.utils.randomSecretKey();
  const publicKey = x25519.getPublicKey(privateKey);
  return {
    keyId,
    publicKey,
    privateKey,
    signature: ed25519.sign(publicKey, identity.sigPrivateKey),
    createdAt: Date.now(),
  };
}

function bundleFrom(
  identity: IdentityKeyPair,
  spk: SignedPreKey,
  userId: string,
  deviceId: string,
): KeyBundle {
  return {
    userId,
    deviceId,
    signalDeviceId: 1,
    identity_dh_key: bytesToB64(identity.dhPublicKey),
    identity_sig_key: bytesToB64(identity.sigPublicKey),
    signed_prekey_id: spk.keyId,
    signed_prekey: bytesToB64(spk.publicKey),
    signed_prekey_sig: bytesToB64(spk.signature),
    one_time_prekey_id: null,
    one_time_prekey: null,
  };
}

async function establishSessions(): Promise<{ alice: Session; bob: Session }> {
  const aliceIdentity = newIdentity();
  const bobIdentity = newIdentity();
  const bobSignedPreKey = signedPreKey(bobIdentity);
  const bobBundle = bundleFrom(
    bobIdentity,
    bobSignedPreKey,
    "bob",
    "bob-device",
  );

  const initiated = await x3dhInitiate(
    aliceIdentity,
    bobBundle,
    "alice-session",
    "conversation-1",
  );
  const responded = await x3dhRespond(
    bobIdentity,
    bobSignedPreKey,
    null,
    initiated.ephemeralPublicKey,
    aliceIdentity.dhPublicKey,
    "bob-session",
    "conversation-1",
    "alice",
    "alice-device",
    1,
  );

  return { alice: initiated.session, bob: responded };
}

describe("double ratchet", () => {
  it("supports alternating turns with DH ratchet steps", async () => {
    const decoder = new TextDecoder();
    const encoder = new TextEncoder();
    const established = await establishSessions();

    const first = await encryptRatchet(
      established.alice,
      encoder.encode("hello bob"),
    );
    const bobAfterFirst = await decryptRatchet(
      established.bob,
      first.ciphertext,
      {
        msgNum: first.msgNum,
        ratchetPub: first.ratchetPub,
        previousChainLen: first.previousChainLen,
        cryptoVersion: first.cryptoVersion,
      },
    );
    expect(decoder.decode(bobAfterFirst.plaintext)).toBe("hello bob");

    const reply = await encryptRatchet(
      bobAfterFirst.updatedSession,
      encoder.encode("hello alice"),
    );
    const aliceAfterReply = await decryptRatchet(
      first.updatedSession,
      reply.ciphertext,
      {
        msgNum: reply.msgNum,
        ratchetPub: reply.ratchetPub,
        previousChainLen: reply.previousChainLen,
        cryptoVersion: reply.cryptoVersion,
      },
    );
    expect(decoder.decode(aliceAfterReply.plaintext)).toBe("hello alice");

    const second = await encryptRatchet(
      aliceAfterReply.updatedSession,
      encoder.encode("second turn"),
    );
    const bobAfterSecond = await decryptRatchet(
      reply.updatedSession,
      second.ciphertext,
      {
        msgNum: second.msgNum,
        ratchetPub: second.ratchetPub,
        previousChainLen: second.previousChainLen,
        cryptoVersion: second.cryptoVersion,
      },
    );
    expect(decoder.decode(bobAfterSecond.plaintext)).toBe("second turn");
  });

  it("decrypts skipped messages out of order", async () => {
    const decoder = new TextDecoder();
    const encoder = new TextEncoder();
    const established = await establishSessions();

    const first = await encryptRatchet(established.alice, encoder.encode("m0"));
    const second = await encryptRatchet(
      first.updatedSession,
      encoder.encode("m1"),
    );
    const third = await encryptRatchet(
      second.updatedSession,
      encoder.encode("m2"),
    );

    const afterFirst = await decryptRatchet(established.bob, first.ciphertext, {
      msgNum: first.msgNum,
      ratchetPub: first.ratchetPub,
      previousChainLen: first.previousChainLen,
      cryptoVersion: first.cryptoVersion,
    });
    expect(decoder.decode(afterFirst.plaintext)).toBe("m0");

    const afterThird = await decryptRatchet(
      afterFirst.updatedSession,
      third.ciphertext,
      {
        msgNum: third.msgNum,
        ratchetPub: third.ratchetPub,
        previousChainLen: third.previousChainLen,
        cryptoVersion: third.cryptoVersion,
      },
    );
    expect(decoder.decode(afterThird.plaintext)).toBe("m2");

    const afterSecond = await decryptRatchet(
      afterThird.updatedSession,
      second.ciphertext,
      {
        msgNum: second.msgNum,
        ratchetPub: second.ratchetPub,
        previousChainLen: second.previousChainLen,
        cryptoVersion: second.cryptoVersion,
      },
    );
    expect(decoder.decode(afterSecond.plaintext)).toBe("m1");
  });

  it("rejects replayed messages", async () => {
    const encoder = new TextEncoder();
    const established = await establishSessions();

    const first = await encryptRatchet(
      established.alice,
      encoder.encode("once"),
    );
    const firstDecrypt = await decryptRatchet(
      established.bob,
      first.ciphertext,
      {
        msgNum: first.msgNum,
        ratchetPub: first.ratchetPub,
        previousChainLen: first.previousChainLen,
        cryptoVersion: first.cryptoVersion,
      },
    );

    await expect(
      decryptRatchet(firstDecrypt.updatedSession, first.ciphertext, {
        msgNum: first.msgNum,
        ratchetPub: first.ratchetPub,
        previousChainLen: first.previousChainLen,
        cryptoVersion: first.cryptoVersion,
      }),
    ).rejects.toThrow(/replay/i);
  });

  it("keeps legacy sessions decryptable via crypto version 1", async () => {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const legacyChain = crypto.getRandomValues(new Uint8Array(32));

    const sender: Session = {
      sessionId: "legacy-sender",
      conversationId: "conversation-legacy",
      peerId: "peer",
      peerDeviceId: "peer-device",
      peerSignalDeviceId: 1,
      version: 1,
      rootKey: crypto.getRandomValues(new Uint8Array(32)),
      sendChainKey: legacyChain.slice(),
      receiveChainKey: null,
      sendMsgNum: 0,
      receiveMsgNum: 0,
    };
    const receiver: Session = {
      ...sender,
      sessionId: "legacy-receiver",
      sendChainKey: null,
      receiveChainKey: legacyChain.slice(),
    };

    const encrypted = await encryptRatchet(sender, encoder.encode("legacy"));
    const decrypted = await decryptRatchet(receiver, encrypted.ciphertext, {
      msgNum: encrypted.msgNum,
      cryptoVersion: encrypted.cryptoVersion,
    });
    expect(decoder.decode(decrypted.plaintext)).toBe("legacy");
  });

  it("advances the send chain on each encrypt", async () => {
    const encoder = new TextEncoder();
    const established = await establishSessions();

    const first = await encryptRatchet(
      established.alice,
      encoder.encode("m0"),
    );
    const second = await encryptRatchet(
      first.updatedSession,
      encoder.encode("m1"),
    );

    expect(first.updatedSession.sendChainKey).not.toEqual(
      second.updatedSession.sendChainKey,
    );
    expect(first.updatedSession.sendMsgNum).toBe(1);
    expect(second.updatedSession.sendMsgNum).toBe(2);
  });

  it("bounds skipped-message storage at the configured maximum", async () => {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const established = await establishSessions();
    const fillerPub = crypto.getRandomValues(new Uint8Array(32));

    established.bob.skippedMessageKeys = Array.from(
      { length: 512 },
      (_, index) => ({
        ratchetPub: fillerPub.slice(),
        msgNum: index,
        messageKey: crypto.getRandomValues(new Uint8Array(32)),
      }),
    );

    const first = await encryptRatchet(established.alice, encoder.encode("m0"));
    const second = await encryptRatchet(
      first.updatedSession,
      encoder.encode("m1"),
    );

    const decrypted = await decryptRatchet(established.bob, second.ciphertext, {
      msgNum: second.msgNum,
      ratchetPub: second.ratchetPub,
      previousChainLen: second.previousChainLen,
      cryptoVersion: second.cryptoVersion,
    });

    expect(decoder.decode(decrypted.plaintext)).toBe("m1");
    expect(decrypted.updatedSession.skippedMessageKeys).toHaveLength(512);
  });
});
