/**
 * Signal Protocol wrapper — main API.
 *
 * Uses @noble/curves (Ed25519 + X25519) and Web Crypto AES-256-GCM.
 * Works in all WebView environments: Tauri, Capacitor, and Web PWA.
 *
 * Typical call order:
 *   1. generateIdentityKey()       — on first account setup
 *   2. generateSignedPreKey(id)    — on setup + rotate weekly
 *   3. generateOneTimePreKeys(100) — on setup + refill when low
 *   4. uploadKeysToServer()        — after generating
 *   5. encryptDm() / decryptDm()   — for each DM message
 *   6. joinChannel(channelId)      — on first join (generates + distributes SenderKey)
 *   7. encryptChannel() / decryptChannel() — for channel messages
 */

import { x25519, ed25519 } from "@noble/curves/ed25519.js";
import { api } from "$api/client.js";
import { authStore } from "$stores/auth.js";
import { get } from "svelte/store";
import * as ks from "./keystore.js";
import { x3dhInitiate, x3dhRespond } from "./x3dh.js";
import { encryptRatchet, decryptRatchet } from "./ratchet.js";
import {
  generateSenderKey,
  encryptWithSenderKey,
  decryptWithSenderKey,
  encryptSenderKeyDist,
  decryptSenderKeyDist,
  packChannelMessage,
  unpackChannelMessage,
  signSenderKeyDistPayload,
  verifySenderKeyDistPayload,
} from "./sender_keys.js";
import type {
  IdentityKeyPair,
  KeyBundle,
  PreKeyPair,
  SignedPreKey,
  Session,
  EncryptedMessage,
  SenderKey,
  SenderKeyRecord,
  SenderKeyDistPayload,
  EncryptedChannelMessage,
} from "./types.js";

export type {
  IdentityKeyPair,
  KeyBundle,
  PreKeyPair,
  SignedPreKey,
  Session,
  EncryptedMessage,
  SenderKey,
  SenderKeyRecord,
  EncryptedChannelMessage,
};
export { backupKeys, restoreKeys } from "./backup.js";

const OPK_BATCH_SIZE = 100;

interface KeyBundleV2Response {
  user_id: string;
  device_id: string;
  signal_device_id: number;
  identity_dh_key: string;
  identity_sig_key: string;
  signed_prekey_id: number;
  signed_prekey: string;
  signed_prekey_sig: string;
  one_time_prekey_id: number | null;
  one_time_prekey: string | null;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function bytesToB64(bytes: Uint8Array): string {
  let binary = "";
  for (let index = 0; index < bytes.length; index += 1) {
    binary += String.fromCharCode(bytes[index]);
  }
  return btoa(binary);
}

function b64ToBytes(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) {
    return false;
  }
  let diff = 0;
  for (let i = 0; i < a.length; i++) {
    diff |= a[i] ^ b[i];
  }
  return diff === 0;
}

function currentSignalDeviceId(): number {
  return get(authStore).device?.signalDeviceId ?? 1;
}

function normalizeKeyBundle(bundle: KeyBundleV2Response): KeyBundle {
  return {
    userId: bundle.user_id,
    deviceId: bundle.device_id,
    signalDeviceId: bundle.signal_device_id,
    identity_dh_key: bundle.identity_dh_key,
    identity_sig_key: bundle.identity_sig_key,
    signed_prekey_id: bundle.signed_prekey_id,
    signed_prekey: bundle.signed_prekey,
    signed_prekey_sig: bundle.signed_prekey_sig,
    one_time_prekey_id: bundle.one_time_prekey_id,
    one_time_prekey: bundle.one_time_prekey,
  };
}

export function dmSessionId(
  conversationId: string,
  peerId: string,
  peerDeviceId: string,
): string {
  return `${conversationId}:${peerId}:${peerDeviceId}`;
}

function stripOneTimePreKey(bundle: KeyBundle): KeyBundle {
  return {
    ...bundle,
    one_time_prekey: null,
    one_time_prekey_id: null,
  };
}

async function fetchKeyBundles(
  userId: string,
  opts: { consumeOpk?: boolean; deviceIds?: string[] } = {},
): Promise<KeyBundle[]> {
  const query = new URLSearchParams();
  if (opts.consumeOpk) {
    query.set("consume_opk", "true");
  }
  if (opts.deviceIds?.length) {
    query.set("device_ids", opts.deviceIds.join(","));
  }
  const suffix = query.size > 0 ? `?${query.toString()}` : "";
  const bundles = await api.get<KeyBundleV2Response[]>(
    `/api/v2/keys/${userId}/bundles${suffix}`,
  );
  if (!bundles.length) {
    throw new Error(`No trusted device bundles for user ${userId}`);
  }
  return bundles.map(normalizeKeyBundle);
}

async function fetchPrimaryKeyBundle(userId: string): Promise<KeyBundle> {
  const bundles = await fetchKeyBundles(userId);
  return bundles[0];
}

async function fetchDeviceKeyBundle(
  userId: string,
  deviceId: string,
): Promise<KeyBundle> {
  const bundles = await fetchKeyBundles(userId);
  const bundle = bundles.find((candidate) => candidate.deviceId === deviceId);
  if (!bundle) {
    throw new Error(`No trusted device bundle for ${userId}/${deviceId}`);
  }
  return bundle;
}

// ─── Key Generation ───────────────────────────────────────────────────────────

/**
 * Generate a fresh identity key pair.
 * Call once on first device registration; persist via keystore.
 */
export async function generateIdentityKey(): Promise<IdentityKeyPair> {
  const dhPrivateKey = x25519.utils.randomSecretKey();
  const dhPublicKey = x25519.getPublicKey(dhPrivateKey);
  const sigPrivateKey = ed25519.utils.randomSecretKey();
  const sigPublicKey = ed25519.getPublicKey(sigPrivateKey);
  return { dhPublicKey, dhPrivateKey, sigPublicKey, sigPrivateKey };
}

/**
 * Generate a signed prekey.
 * @param keyId  — monotonic integer, e.g. 1 for first key
 */
export async function generateSignedPreKey(
  identity: IdentityKeyPair,
  keyId: number,
): Promise<SignedPreKey> {
  const privateKey = x25519.utils.randomSecretKey();
  const publicKey = x25519.getPublicKey(privateKey);
  // Sign the X25519 public key with the Ed25519 identity signing key
  const signature = ed25519.sign(publicKey, identity.sigPrivateKey);
  return { keyId, publicKey, privateKey, signature, createdAt: Date.now() };
}

/**
 * Generate a batch of one-time prekeys.
 * @param count   — number of keys (default 100)
 * @param startId — first key id in the batch
 */
export async function generateOneTimePreKeys(
  count = 100,
  startId = 1,
): Promise<PreKeyPair[]> {
  return Array.from({ length: count }, (_, i) => {
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    return { keyId: startId + i, publicKey, privateKey };
  });
}

async function ensureLocalSignedPreKey(
  identity: IdentityKeyPair,
): Promise<SignedPreKey> {
  const existing = await ks.loadLatestSignedPreKey();
  if (existing) return existing;

  const spk = await generateSignedPreKey(identity, 1);
  await ks.storeSignedPreKey(spk);
  return spk;
}

async function ensureLocalPreKeys(
  targetCount = OPK_BATCH_SIZE,
): Promise<PreKeyPair[]> {
  const existing = await ks.listPreKeys();
  if (existing.length >= targetCount) return existing;

  const nextId =
    existing.reduce((maxId, prekey) => Math.max(maxId, prekey.keyId), 0) + 1;
  const newKeys = await generateOneTimePreKeys(
    targetCount - existing.length,
    nextId,
  );
  for (const pk of newKeys) {
    await ks.storePreKey(pk);
  }
  return existing.concat(newKeys);
}

// ─── Key Upload ───────────────────────────────────────────────────────────────

/** Upload identity key to the server. Call once after generateIdentityKey(). */
export async function uploadIdentityKey(
  identity: IdentityKeyPair,
): Promise<void> {
  await api.post("/api/v2/keys/identity", {
    device_id: currentSignalDeviceId(),
    dh_public_key: bytesToB64(identity.dhPublicKey),
    signing_public_key: bytesToB64(identity.sigPublicKey),
  });
}

/** Upload a signed prekey to the server. */
export async function uploadSignedPreKey(spk: SignedPreKey): Promise<void> {
  await api.post("/api/v2/keys/signed-prekey", {
    device_id: currentSignalDeviceId(),
    key_id: spk.keyId,
    public_key: bytesToB64(spk.publicKey),
    signature: bytesToB64(spk.signature),
  });
}

/** Upload a batch of one-time prekeys to the server. */
export async function uploadOneTimePreKeys(
  prekeys: PreKeyPair[],
): Promise<void> {
  await api.post("/api/v2/keys/one-time-prekeys", {
    device_id: currentSignalDeviceId(),
    keys: prekeys.map((k) => ({
      key_id: k.keyId,
      public_key: bytesToB64(k.publicKey),
    })),
  });
}

// ─── Full Setup ───────────────────────────────────────────────────────────────

/**
 * First-time key setup: generate all keys, persist locally, upload to server.
 * Idempotent — if keys already exist in the keystore, this is a no-op.
 */
export async function setupKeys(): Promise<void> {
  let identity = await ks.loadIdentityKeyPair();
  const bootstrapComplete = await ks.loadSignalBootstrapComplete();
  if (identity && bootstrapComplete) return;

  if (!identity) {
    identity = await generateIdentityKey();
    await ks.storeIdentityKeyPair(identity);
  }

  const spk = await ensureLocalSignedPreKey(identity);
  const prekeys = await ensureLocalPreKeys();

  await uploadIdentityKey(identity);
  await uploadSignedPreKey(spk);
  if (prekeys.length > 0) {
    await uploadOneTimePreKeys(prekeys);
  }
  await ks.storeSignalBootstrapComplete(true);
}

/**
 * Check OPK count on server and replenish if below threshold.
 * Call after setupKeys() on each app start.
 */
export async function replenishPreKeysIfNeeded(): Promise<void> {
  const { count, low } = await api.get<{ count: number; low: boolean }>(
    "/api/v2/keys/one-time-prekey-count",
  );
  if (!low) return;

  const identity = await ks.loadIdentityKeyPair();
  if (!identity) return;

  const existing = await ks.listPreKeys();
  const startId =
    existing.reduce((maxId, prekey) => Math.max(maxId, prekey.keyId), 0) + 1;
  const newKeys = await generateOneTimePreKeys(OPK_BATCH_SIZE, startId);
  for (const pk of newKeys) {
    await ks.storePreKey(pk);
  }
  await ks.storeSignalBootstrapComplete(false);
  await uploadOneTimePreKeys(newKeys);
  await ks.storeSignalBootstrapComplete(true);
  console.debug(`[Signal] Replenished ${newKeys.length} OPKs (was ${count})`);
}

// ─── Encrypt DM ───────────────────────────────────────────────────────────────

/**
 * Encrypt a plaintext message for a DM conversation.
 * Emits one envelope per active peer device plus one envelope for every
 * trusted sender device so sent history survives across the account's devices.
 */
export async function encryptDm(
  conversationId: string,
  peerId: string,
  plaintext: string,
): Promise<{
  envelopes: Array<{
    recipientUserId: string;
    recipientDeviceId: string;
    ciphertext: string;
    ephemeralKey?: string;
    opkId?: number;
    msgNum: number;
    ratchetPub?: string;
    previousChainLen?: number;
    cryptoVersion: number;
  }>;
}> {
  interface DmTarget {
    userId: string;
    deviceId: string;
    initBundle?: KeyBundle;
  }

  const auth = get(authStore);
  const myUserId = auth.user?.id;
  const myDeviceId = auth.device?.id;
  if (!myUserId || !myDeviceId) {
    throw new Error("No active trusted device");
  }

  const existingPeerSessions = await ks.listSessionsForPeer(
    conversationId,
    peerId,
  );
  const existingOwnSessions = await ks.listSessionsForPeer(
    conversationId,
    myUserId,
  );
  const existingPeerDevices = new Set(
    existingPeerSessions
      .filter((session) => session.version === 2)
      .map((session) => session.peerDeviceId),
  );
  const existingOwnDevices = new Set(
    existingOwnSessions
      .filter((session) => session.version === 2)
      .map((session) => session.peerDeviceId),
  );
  const advertisedPeerBundles = await fetchKeyBundles(peerId);
  const missingPeerDevices = advertisedPeerBundles
    .filter((bundle) => !existingPeerDevices.has(bundle.deviceId))
    .map((bundle) => bundle.deviceId);

  let initPeerBundleMap = new Map<string, KeyBundle>();
  if (missingPeerDevices.length > 0) {
    const initBundles = await fetchKeyBundles(peerId, {
      consumeOpk: true,
      deviceIds: missingPeerDevices,
    });
    initPeerBundleMap = new Map(
      initBundles.map((bundle) => [bundle.deviceId, bundle]),
    );
  }

  const advertisedOwnBundles = await fetchKeyBundles(myUserId);
  const missingOwnDevices = advertisedOwnBundles
    .filter(
      (bundle) =>
        bundle.deviceId !== myDeviceId &&
        !existingOwnDevices.has(bundle.deviceId),
    )
    .map((bundle) => bundle.deviceId);

  let initOwnBundleMap = new Map<string, KeyBundle>();
  if (missingOwnDevices.length > 0) {
    const initBundles = await fetchKeyBundles(myUserId, {
      consumeOpk: true,
      deviceIds: missingOwnDevices,
    });
    initOwnBundleMap = new Map(
      initBundles.map((bundle) => [bundle.deviceId, bundle]),
    );
  }

  const targets: DmTarget[] = advertisedPeerBundles.map((bundle) => ({
    userId: bundle.userId,
    deviceId: bundle.deviceId,
    initBundle: existingPeerDevices.has(bundle.deviceId)
      ? undefined
      : (initPeerBundleMap.get(bundle.deviceId) ?? stripOneTimePreKey(bundle)),
  }));

  targets.push(
    ...advertisedOwnBundles.map((bundle) => ({
      userId: bundle.userId,
      deviceId: bundle.deviceId,
      initBundle: existingOwnDevices.has(bundle.deviceId)
        ? undefined
        : bundle.deviceId === myDeviceId
          ? stripOneTimePreKey(bundle)
          : (initOwnBundleMap.get(bundle.deviceId) ??
            stripOneTimePreKey(bundle)),
    })),
  );

  if (targets.length > 16) {
    throw new Error("Too many trusted device targets for DM envelope fanout");
  }

  const envelopes: Array<{
    recipientUserId: string;
    recipientDeviceId: string;
    ciphertext: string;
    ephemeralKey?: string;
    opkId?: number;
    msgNum: number;
    ratchetPub?: string;
    previousChainLen?: number;
    cryptoVersion: number;
  }> = [];

  for (const target of targets) {
    const sessionKey = dmSessionId(
      conversationId,
      target.userId,
      target.deviceId,
    );
    let session = await ks.loadSession(sessionKey);
    if (session && session.version !== 2) {
      await ks.deleteSession(sessionKey);
      session = null;
    }

    let ephemeralKey: string | undefined;
    let opkId: number | undefined;

    if (!session) {
      const identity = await ks.loadIdentityKeyPair();
      if (!identity)
        throw new Error("No identity key - call setupKeys() first");
      if (!target.initBundle) {
        throw new Error("Missing initial bundle for device " + target.deviceId);
      }

      if (target.userId !== myUserId) {
        const peerFp = await fingerprintKeys(
          b64ToBytes(target.initBundle.identity_dh_key),
          b64ToBytes(target.initBundle.identity_sig_key),
        );
        await requirePeerKeyUnchanged(target.userId, target.deviceId, peerFp);
      }

      const result = await x3dhInitiate(
        identity,
        target.initBundle,
        sessionKey,
        conversationId,
      );
      session = result.session;
      ephemeralKey = bytesToB64(result.ephemeralPublicKey);
      opkId = result.usedOpkId ?? undefined;
    }

    const plainBytes = new TextEncoder().encode(plaintext);
    const {
      ciphertext,
      msgNum,
      ratchetPub,
      previousChainLen,
      cryptoVersion,
      updatedSession,
    } = await encryptRatchet(session, plainBytes);
    await ks.storeSession(updatedSession);

    envelopes.push({
      recipientUserId: target.userId,
      recipientDeviceId: target.deviceId,
      ciphertext: bytesToB64(ciphertext),
      ephemeralKey,
      opkId,
      msgNum,
      ratchetPub: ratchetPub ? bytesToB64(ratchetPub) : undefined,
      previousChainLen,
      cryptoVersion,
    });
  }

  return { envelopes };
}

/**
 * Decrypt a received DM message.
 * On first message from a peer, reconstructs the X3DH session (responder side).
 */
export async function decryptDm(
  conversationId: string,
  peerId: string,
  peerDeviceId: string,
  peerSignalDeviceId: number,
  msg: {
    ciphertext: string;
    ephemeral_key?: string | null;
    opk_id?: number | null;
    msg_num?: number | null;
    ratchet_pub?: string | null;
    previous_chain_len?: number | null;
    crypto_version?: number | null;
  },
): Promise<string> {
  const sessionKey = dmSessionId(conversationId, peerId, peerDeviceId);
  let session = await ks.loadSession(sessionKey);
  const cipherBytes = b64ToBytes(msg.ciphertext);

  // When establishing a new X3DH session we defer the fingerprint check until
  // AFTER decryptRatchet succeeds.  Checking eagerly (before the ratchet step)
  // stores the peer's current bundle fingerprint even when the ratchet
  // subsequently fails (e.g. old messages whose OPK was consumed by a previous
  // session). That stale fingerprint then causes "Peer identity changed" errors
  // the next time encryptDm is called after the peer regenerated their keys.
  let pendingFpBundle: KeyBundle | null = null;

  if (!session) {
    if (!msg.ephemeral_key) {
      throw new Error("No session and no ephemeral key - cannot decrypt");
    }

    const identity = await ks.loadIdentityKeyPair();
    if (!identity) throw new Error("No identity key");

    const spk = await ks.loadLatestSignedPreKey();
    if (!spk) throw new Error("No signed prekey");

    const opk = msg.opk_id != null ? await ks.loadPreKey(msg.opk_id) : null;
    const ephemeralPubKey = b64ToBytes(msg.ephemeral_key);
    const senderBundles = await fetchKeyBundles(peerId);
    const senderBundle =
      senderBundles.find((bundle) => bundle.deviceId === peerDeviceId) ??
      senderBundles.find(
        (bundle) => bundle.signalDeviceId === peerSignalDeviceId,
      ) ??
      senderBundles[0];
    const senderDhPub = b64ToBytes(senderBundle.identity_dh_key);

    session = await x3dhRespond(
      identity,
      spk,
      opk,
      ephemeralPubKey,
      senderDhPub,
      sessionKey,
      conversationId,
      peerId,
      peerDeviceId,
      peerSignalDeviceId,
    );
    pendingFpBundle = senderBundle;

    if (msg.opk_id != null) {
      await ks.deletePreKey(msg.opk_id);
    }
  }

  const { plaintext, updatedSession } = await decryptRatchet(
    session,
    cipherBytes,
    {
      msgNum: msg.msg_num ?? undefined,
      ratchetPub: msg.ratchet_pub ? b64ToBytes(msg.ratchet_pub) : null,
      previousChainLen: msg.previous_chain_len ?? null,
      cryptoVersion: msg.crypto_version ?? 1,
    },
  );

  // Verify sender fingerprint only after the ratchet step succeeds, and before
  // committing the session. A throw here prevents a changed-key session from
  // being persisted, so the next message triggers a fresh X3DH check.
  if (pendingFpBundle) {
    const senderFp = await fingerprintKeys(
      b64ToBytes(pendingFpBundle.identity_dh_key),
      b64ToBytes(pendingFpBundle.identity_sig_key),
    );
    await requirePeerKeyUnchanged(peerId, pendingFpBundle.deviceId, senderFp);
  }

  await ks.storeSession(updatedSession);
  return new TextDecoder().decode(plaintext);
}

/**
 * Encrypt a message for a channel using the Sender Key ratchet.
 * Returns the packed wire ciphertext (sig || ct, base64) and the ratchet iteration.
 * Throws if no SenderKey exists for this channel — call joinChannel() first.
 */
export async function encryptChannel(
  channelId: string,
  plaintext: string,
): Promise<{ wireCiphertext: string; iteration: number }> {
  const key = await ks.loadSenderKey(channelId);
  if (!key)
    throw new Error(
      `No SenderKey for channel ${channelId} — call joinChannel() first`,
    );

  const plainBytes = new TextEncoder().encode(plaintext);
  const { encrypted, updatedKey } = await encryptWithSenderKey(key, plainBytes);
  await ks.storeSenderKey(updatedKey);
  return {
    wireCiphertext: packChannelMessage(encrypted),
    iteration: encrypted.iteration,
  };
}

/**
 * Decrypt a received channel message from the wire format.
 * The ciphertext field must be base64(sig_64_bytes || aes_ct_bytes) as packed by encryptChannel.
 * Throws if no SenderKeyRecord exists for (channelId, senderId).
 */
export async function decryptChannel(
  channelId: string,
  senderId: string,
  senderDeviceId: string,
  msg: { ciphertext: string; msg_num: number | null },
  opts: { allowHistorical?: boolean } = {},
): Promise<string> {
  const record = await ks.loadReceiverKey(channelId, senderId, senderDeviceId);
  if (!record) {
    throw new Error(
      `No SenderKey for sender ${senderId}/${senderDeviceId} in channel ${channelId}`,
    );
  }

  const encrypted = unpackChannelMessage(msg.ciphertext, msg.msg_num ?? 0);
  const { plaintext, updatedRecord } = await decryptWithSenderKey(
    record,
    encrypted,
    opts,
  );
  await ks.storeReceiverKey(updatedRecord);
  return new TextDecoder().decode(plaintext);
}

/**
 * Join a channel: generate a SenderKey, distribute it to all existing member devices,
 * and fetch any pending SenderKey distributions from other members.
 *
 * Call this once when the user first joins (or after a server restart wipes local keys).
 */
export async function joinChannel(channelId: string): Promise<void> {
  const identity = await ks.loadIdentityKeyPair();
  if (!identity) throw new Error("No identity key - call setupKeys() first");
  const myUserId = get(authStore).user?.id;
  const myDeviceId = get(authStore).device?.id;
  if (!myUserId || !myDeviceId) throw new Error("No active trusted device");

  const senderKey = generateSenderKey(channelId);
  await ks.storeSenderKey(senderKey);

  // Store a receiver key for self so loadChannelMessages() can decrypt our
  // own historical messages after re-navigation (decryptChannel uses
  // loadReceiverKey for all senders including ourselves).
  await ks.storeReceiverKey({
    channelId,
    senderId: myUserId,
    senderDeviceId: myDeviceId,
    chainKey: senderKey.chainKey,
    signingPubKey: senderKey.signingPubKey,
    iteration: senderKey.iteration,
    initialChainKey: senderKey.chainKey,
    initialIteration: senderKey.iteration,
  });

  const members = await api.get<Array<{ user_id: string; username: string }>>(
    "/api/v1/channels/" + channelId + "/members",
  );

  const distPayload: SenderKeyDistPayload = signSenderKeyDistPayload(
    {
      channelId,
      senderUserId: myUserId,
      senderDeviceId: myDeviceId,
      chainKey: bytesToB64(senderKey.chainKey),
      signingPubKey: bytesToB64(senderKey.signingPubKey),
      iteration: senderKey.iteration,
    },
    identity,
  );

  const distributions: Array<{
    to_user_id: string;
    to_device_id: string;
    ciphertext: string;
    ek_public: string;
  }> = [];

  for (const member of members) {
    let recipientBundles: KeyBundle[];
    try {
      recipientBundles = await fetchKeyBundles(member.user_id);
    } catch {
      console.warn(
        "[Signal] Could not fetch key bundles for " +
          member.user_id +
          " - skipping",
      );
      continue;
    }

    for (const recipientBundle of recipientBundles) {
      if (
        member.user_id === myUserId &&
        recipientBundle.deviceId === myDeviceId
      ) {
        continue;
      }

      const recipientDhPub = b64ToBytes(recipientBundle.identity_dh_key);
      const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(
        distPayload,
        recipientDhPub,
        identity.dhPublicKey,
      );
      distributions.push({
        to_user_id: member.user_id,
        to_device_id: recipientBundle.deviceId,
        ciphertext: bytesToB64(ciphertext),
        ek_public: bytesToB64(ephemeralKey),
      });
    }
  }

  if (distributions.length > 0) {
    await api.post("/api/v1/channels/" + channelId + "/sender-key-dist", {
      distributions,
    });
  }

  await fetchAndStorePendingDists(channelId, identity);
}

/**
 * Fetch any pending SenderKey distributions for this channel and persist them.
 * Call on channel open to catch distributions delivered while offline.
 */
export async function fetchPendingKeyDists(channelId: string): Promise<void> {
  const identity = await ks.loadIdentityKeyPair();
  if (!identity) return;
  await fetchAndStorePendingDists(channelId, identity);
}

async function fetchAndStorePendingDists(
  channelId: string,
  identity: IdentityKeyPair,
): Promise<void> {
  const dists = await api
    .get<
      Array<{
        from_user: string;
        from_device_id?: string | null;
        ciphertext: string;
        ek_public: string;
      }>
    >("/api/v1/channels/" + channelId + "/sender-key-dist")
    .catch(
      () =>
        [] as Array<{
          from_user: string;
          from_device_id?: string | null;
          ciphertext: string;
          ek_public: string;
        }>,
    );

  for (const dist of dists) {
    try {
      const ct = b64ToBytes(dist.ciphertext);
      const ek = b64ToBytes(dist.ek_public);
      // Require sender device ID — distributions without it cannot be authenticated
      const senderDeviceId = dist.from_device_id ?? null;
      if (!senderDeviceId) {
        console.warn(
          "[Signal] Skipping key dist with no from_device_id from " +
            dist.from_user,
        );
        continue;
      }
      // Fetch sender bundle to bind ECIES to sender identity (M-01)
      const senderBundle = await fetchDeviceKeyBundle(
        dist.from_user,
        senderDeviceId,
      );
      const senderDhPub = b64ToBytes(senderBundle.identity_dh_key);
      const payload = await decryptSenderKeyDist(
        ct,
        ek,
        identity.dhPrivateKey,
        identity.dhPublicKey,
        senderDhPub,
      );
      if (payload.identitySignature) {
        verifySenderKeyDistPayload(
          payload,
          b64ToBytes(senderBundle.identity_sig_key),
          dist.from_user,
          senderDeviceId,
        );
      }
      const incomingChainKey = b64ToBytes(payload.chainKey);
      const incomingSigningKey = b64ToBytes(payload.signingPubKey);
      const existing = await ks.loadReceiverKey(
        payload.channelId,
        dist.from_user,
        senderDeviceId,
      );

      if (
        existing &&
        bytesEqual(existing.signingPubKey, incomingSigningKey) &&
        existing.iteration > payload.iteration
      ) {
        await ks.storeReceiverKey({
          ...existing,
          initialChainKey: existing.initialChainKey ?? incomingChainKey,
          initialIteration: existing.initialIteration ?? payload.iteration,
        });
        continue;
      }

      const record: SenderKeyRecord = {
        channelId: payload.channelId,
        senderId: dist.from_user,
        senderDeviceId: senderDeviceId,
        chainKey: incomingChainKey,
        signingPubKey: incomingSigningKey,
        iteration: payload.iteration,
        initialChainKey: incomingChainKey,
        initialIteration: payload.iteration,
      };
      await ks.storeReceiverKey(record);
    } catch (e) {
      console.warn(
        "[Signal] Failed to decrypt SenderKey dist from " +
          dist.from_user +
          ":",
        e,
      );
    }
  }
}

/**
 * Prepare a channel for messaging (idempotent).
 * - Generates and distributes a SenderKey if none exists locally yet.
 * - Fetches any pending SenderKey distributions from other members regardless.
 * Safe to call on every channel open — will not regenerate if key already exists.
 */
export async function prepareChannel(channelId: string): Promise<void> {
  const existing = await ks.loadSenderKey(channelId);
  if (!existing) {
    await joinChannel(channelId);
  } else {
    await fetchPendingKeyDists(channelId);
  }
}

// ─── Safety Numbers ───────────────────────────────────────────────────────────

/**
 * Compute a human-readable fingerprint of an identity key (6 groups of 5 decimal digits).
 * Algorithm: SHA-256(dhPub || sigPub) → first 30 bytes → 6 × uint40 mod 100000.
 */
export async function fingerprintKeys(
  dhPub: Uint8Array,
  sigPub: Uint8Array,
): Promise<string> {
  const combined = new Uint8Array(64);
  combined.set(dhPub, 0);
  combined.set(sigPub, 32);
  const hash = await crypto.subtle.digest("SHA-256", combined);
  const bytes = new Uint8Array(hash);
  const groups: string[] = [];
  for (let i = 0; i < 30; i += 5) {
    let val = 0n;
    for (let j = 0; j < 5; j++) val = (val << 8n) | BigInt(bytes[i + j]);
    groups.push((val % 100000n).toString().padStart(5, "0"));
  }
  return groups.join(" ");
}

/** Get own identity fingerprint from local keystore. Returns null if not yet set up. */
export async function getOwnFingerprint(): Promise<string | null> {
  const identity = await ks.loadIdentityKeyPair();
  if (!identity) return null;
  return fingerprintKeys(identity.dhPublicKey, identity.sigPublicKey);
}

/** Fetch a peer's identity keys from the server and compute their fingerprint. */
export async function fetchPeerFingerprint(peerId: string): Promise<string> {
  const bundle = await fetchPrimaryKeyBundle(peerId);
  return fingerprintKeys(
    b64ToBytes(bundle.identity_dh_key),
    b64ToBytes(bundle.identity_sig_key),
  );
}

/** Returns 'new' | 'unchanged' | 'changed'. Persists fingerprint in the scoped signal store. */
async function checkAndStorePeerKey(
  peerId: string,
  peerDeviceId: string,
  fp: string,
): Promise<"new" | "unchanged" | "changed"> {
  const stored = await ks.loadPeerFingerprint(peerId, peerDeviceId);
  await ks.storePeerFingerprint(peerId, peerDeviceId, fp);
  if (!stored) return "new";
  if (stored === fp) return "unchanged";
  await ks.storePeerKeyChanged(peerId, true);
  return "changed";
}

async function requirePeerKeyUnchanged(
  peerId: string,
  peerDeviceId: string,
  fp: string,
): Promise<void> {
  if ((await checkAndStorePeerKey(peerId, peerDeviceId, fp)) === "changed") {
    throw new Error(
      "Peer identity changed; verify safety numbers before sending more messages",
    );
  }
}

/**
 * Handle a real-time key_dist WS event.
 * Decrypts the SenderKey distribution and stores it immediately.
 */
export async function receiveSenderKeyDist(event: {
  channel_id: string;
  from_user: string;
  from_device_id?: string | null;
  ciphertext: string;
  ek_public: string;
}): Promise<void> {
  const identity = await ks.loadIdentityKeyPair();
  if (!identity) return;

  try {
    const ct = b64ToBytes(event.ciphertext);
    const ek = b64ToBytes(event.ek_public);
    // Require sender device ID — distributions without it cannot be authenticated
    const senderDeviceId = event.from_device_id ?? null;
    if (!senderDeviceId) {
      console.warn(
        "[Signal] Skipping key_dist with no from_device_id from " +
          event.from_user,
      );
      return;
    }
    // Fetch sender bundle to bind ECIES to sender identity (M-01)
    const senderBundle = await fetchDeviceKeyBundle(
      event.from_user,
      senderDeviceId,
    );
    const senderDhPub = b64ToBytes(senderBundle.identity_dh_key);
    const payload = await decryptSenderKeyDist(
      ct,
      ek,
      identity.dhPrivateKey,
      identity.dhPublicKey,
      senderDhPub,
    );
    if (payload.identitySignature) {
      verifySenderKeyDistPayload(
        payload,
        b64ToBytes(senderBundle.identity_sig_key),
        event.from_user,
        senderDeviceId,
      );
    }
    const incomingChainKey = b64ToBytes(payload.chainKey);
    const incomingSigningKey = b64ToBytes(payload.signingPubKey);
    const existing = await ks.loadReceiverKey(
      payload.channelId,
      event.from_user,
      senderDeviceId,
    );

    if (
      existing &&
      bytesEqual(existing.signingPubKey, incomingSigningKey) &&
      existing.iteration > payload.iteration
    ) {
      await ks.storeReceiverKey({
        ...existing,
        initialChainKey: existing.initialChainKey ?? incomingChainKey,
        initialIteration: existing.initialIteration ?? payload.iteration,
      });
      console.debug(
        "[Signal] Refreshed SenderKey seed for " +
          event.from_user +
          "/" +
          senderDeviceId +
          " in channel " +
          event.channel_id,
      );
      return;
    }

    const record: SenderKeyRecord = {
      channelId: payload.channelId,
      senderId: event.from_user,
      senderDeviceId,
      chainKey: incomingChainKey,
      signingPubKey: incomingSigningKey,
      iteration: payload.iteration,
      initialChainKey: incomingChainKey,
      initialIteration: payload.iteration,
    };
    await ks.storeReceiverKey(record);
    console.debug(
      "[Signal] Received SenderKey from " +
        event.from_user +
        "/" +
        senderDeviceId +
        " for channel " +
        event.channel_id,
    );
  } catch (e) {
    console.warn(
      "[Signal] Failed to process key_dist from " + event.from_user + ":",
      e,
    );
  }
}

/**
 * Handle a real-time key_dist_request WS event.
 * A new device just joined the channel and needs our sender key.
 * Encrypts and posts our current SenderKey distribution to that device.
 */
export async function handleKeyDistRequest(event: {
  channel_id: string;
  requester_user_id: string;
  requester_device_id: string;
}): Promise<void> {
  const identity = await ks.loadIdentityKeyPair();
  if (!identity) return;
  const myUserId = get(authStore).user?.id;
  const myDeviceId = get(authStore).device?.id;
  if (!myUserId || !myDeviceId) return;
  // Don't send to ourselves
  if (
    event.requester_user_id === myUserId &&
    event.requester_device_id === myDeviceId
  )
    return;

  const senderKey = await ks.loadSenderKey(event.channel_id);
  if (!senderKey) return; // We don't have a key for this channel

  const distPayload: SenderKeyDistPayload = signSenderKeyDistPayload(
    {
      channelId: event.channel_id,
      senderUserId: myUserId,
      senderDeviceId: myDeviceId,
      chainKey: bytesToB64(senderKey.chainKey),
      signingPubKey: bytesToB64(senderKey.signingPubKey),
      iteration: senderKey.iteration,
    },
    identity,
  );

  try {
    const recipientBundle = await fetchDeviceKeyBundle(
      event.requester_user_id,
      event.requester_device_id,
    );
    const recipientDhPub = b64ToBytes(recipientBundle.identity_dh_key);
    const { ciphertext, ephemeralKey } = await encryptSenderKeyDist(
      distPayload,
      recipientDhPub,
      identity.dhPublicKey,
    );
    await api.post(
      "/api/v1/channels/" + event.channel_id + "/sender-key-dist",
      {
        distributions: [
          {
            to_user_id: event.requester_user_id,
            to_device_id: event.requester_device_id,
            ciphertext: bytesToB64(ciphertext),
            ek_public: bytesToB64(ephemeralKey),
          },
        ],
      },
    );
    console.debug(
      "[Signal] Redistributed sender key for channel " +
        event.channel_id +
        " to device " +
        event.requester_device_id,
    );
  } catch (e) {
    console.warn("[Signal] Failed to redistribute sender key:", e);
  }
}
