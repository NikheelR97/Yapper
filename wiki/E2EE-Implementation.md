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

Bob's key bundle (fetched from /api/v2/keys/bundle/:bob_id):
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
2. If not: call prepareChannel() → POST /api/v2/keys/sender-key-dist
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

Users can back up their Signal keystore with a recovery passphrase:

```
1. Derive a 256-bit AES-GCM key from the passphrase via PBKDF2-HMAC-SHA256
   (1,200,000 iterations, random 16-byte salt)
2. Encrypt the keystore snapshot with AES-256-GCM
3. POST the encrypted blob to /api/v2/keys/backup
4. Server stores opaque ciphertext — cannot read it
```

The passphrase must be 12–1024 characters and contain both letters and numbers, plus either an uppercase letter or a special character. On a new device, the user enters their passphrase to restore the keystore.

The 1,200,000 iteration count exceeds the OWASP 2023 floor of ≥600,000 for PBKDF2-HMAC-SHA256 by 2×.

Implementation: `frontend/src/lib/signal/backup.ts` (`PBKDF2_ITERS = 1_200_000`)

---

## Safety Numbers

Safety numbers are derived deterministically from a peer's identity keys:

```
fingerprint = SHA-256(identity_dhPub || identity_sigPub)
            → first 30 bytes
            → 6 × uint40 (mod 100000)
displayed as 6 groups of 5 decimal digits (30 digits total)
```

The concatenation order is fixed at `dhPub || sigPub` for **every** peer, so both sides of a conversation always derive the same fingerprint without any sort/lex coordination. Users can compare fingerprints out-of-band to verify there is no man-in-the-middle.

The display form provides ≈99.6 bits of collision resistance (30 decimal digits × log₂(10) ≈ 99.6 bits), which is sufficient for human-comparable MITM verification.

Implementation: `frontend/src/lib/signal/index.ts` (`fingerprintKeys`), rendered by `frontend/src/lib/components/chat/SafetyNumbers.svelte`.

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
