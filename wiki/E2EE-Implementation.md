# E2EE Implementation

Yapper uses a Signal-style end-to-end encryption scheme implemented entirely in the frontend using `@noble/curves` and `@noble/hashes`. **The server never has access to plaintext messages or media.**

## Why not libsignal?

`@signalapp/libsignal-client` is NAPI-only and cannot run in a WebView (Tauri or Capacitor). We use `@noble/curves` (Ed25519, X25519) and `@noble/hashes` (HKDF, HMAC-SHA256), which are pure TypeScript, fully audited, and run on every platform.

---

## Direct Messages — X3DH + Double Ratchet

### Initial key exchange (X3DH)

When Alice sends the first message to Bob:

```
Alice has:
  IKa  — identity key pair (Ed25519 → X25519 for DH)
  EKa  — ephemeral key pair (X25519, generated per session)

Bob's key bundle (fetched from /api/v1/keys/bundle/:bob_id):
  IKb  — Bob's identity key
  SPKb — Bob's signed prekey + signature
  OPKb — One-time prekey (consumed on use)

DH computations:
  DH1 = DH(IKa, SPKb)
  DH2 = DH(EKa, IKb)
  DH3 = DH(EKa, SPKb)
  DH4 = DH(EKa, OPKb)   ← optional but used when available

Master secret = HKDF(DH1 || DH2 || DH3 || DH4)
```

Implementation: `frontend/src/lib/signal/x3dh.ts`

### Double ratchet

After X3DH establishes the root key, every message uses the double ratchet:

1. **Symmetric ratchet** — HMAC-SHA256 chain key advances per message
2. **DH ratchet** — new DH exchange on reply, providing forward secrecy

Encryption: AES-256-GCM with a per-message key derived from the chain.

Implementation: `frontend/src/lib/signal/ratchet.ts`

### Message envelope (v2)

```json
{
  "registration_id": 1,
  "device_id": 1,
  "ciphertext": "<base64>",
  "message_number": 42,
  "one_time_prekey_id": 7
}
```

The `message_number` enables out-of-order decryption with a skipped-message-key cache.

---

## Group Channels — Sender Keys

Group channels use Sender Keys, which are more efficient than pairwise X3DH for N-party messaging.

### Key structure

Each sender has:
- **Chain key** — HMAC-SHA256, ratchets forward per message
- **Signing key** — Ed25519, signs each ciphertext
- **Distribution message** — ECIES-encrypted to each recipient's identity key

### Sending a channel message

```
1. Derive message key from chain key (HMAC-SHA256)
2. Encrypt plaintext with AES-256-GCM using message key
3. Sign the ciphertext with Ed25519 signing key
4. Wire format: base64(signature_64_bytes || aes_ciphertext)
5. Advance chain key
```

### Joining a channel

```
1. Check if sender key exists in keystore
2. If not: call prepareChannel() → POST /api/v1/keys/sender-key-dist
3. Server distributes the sender's key bundle to all channel members
4. Each member decrypts with their identity key (ECIES)
```

`prepareChannel()` is idempotent — safe to call on every channel view.

Implementation: `frontend/src/lib/signal/sender_keys.ts`

---

## Media Encryption

Before uploading any file to Cloudflare R2:

```
1. Generate random AES-256-GCM key + IV
2. Encrypt file bytes → ciphertext
3. Upload ciphertext to R2 via presigned URL
4. Embed { key, iv, r2_url } in the encrypted message payload
```

The R2 URL is public but the file is indecipherable without the key, which only exists in the message ciphertext.

Implementation: `frontend/src/lib/signal/mediaEncrypt.ts`

---

## Keystore

All Signal key material is persisted in **IndexedDB** via the `idb` library.

```
IndexedDB: "yapper-signal"
  ├── identityKeyPair        (IK — Ed25519)
  ├── registrationId
  ├── signedPreKeyPair       (SPK — X25519)
  ├── oneTimePreKeys[]       (OPKs — X25519)
  ├── sessions{}             (per-recipient ratchet state)
  ├── senderKeys{}           (per-channel sender key state)
  └── skippedMessageKeys{}   (out-of-order decryption cache)
```

The keystore is initialised once at login. A double-init guard prevents corruption on hot-reload.

Implementation: `frontend/src/lib/signal/keystore.ts`

---

## Key Backup

Users can back up their Signal keystore with a PIN:

```
1. Derive encryption key from PIN via PBKDF2 (100K iterations, SHA-256)
2. Encrypt keystore JSON with AES-256-GCM
3. POST encrypted blob to /api/v1/keys/backup
4. Server stores opaque ciphertext — cannot read it
```

On a new device, the user enters their PIN to restore the keystore.

Implementation: `frontend/src/lib/signal/backup.ts`

---

## Safety Numbers

Safety numbers are derived from both parties' identity keys:

```
safety_number = first 60 digits of SHA-512(IKa || IKb)
displayed as 12 groups of 5 digits
```

Users can compare safety numbers out-of-band to verify there is no man-in-the-middle. Implementation: `frontend/src/lib/components/settings/PrivacySafety.svelte`.

---

## What the server stores

| Data | Server sees |
|------|-------------|
| Messages | Opaque ciphertext blobs only |
| Media | Encrypted bytes only |
| Identity keys | Public keys only (needed for key distribution) |
| Sender key distributions | ECIES-encrypted blobs (per-recipient, opaque) |
| Key backup | PIN-encrypted blob (server cannot decrypt) |
| Metadata | Sender ID, recipient ID, timestamp, message number |

The server handles routing and storage but cannot read any message content or media.
