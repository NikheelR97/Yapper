# E2EE & Security Model

## Principles

1. **The server never sees plaintext.** All message content is encrypted on-device before transmission.
2. **Parental controls operate on metadata only.** Parents can see who their child is communicating with and what servers they are trying to join — never the content of any message.
3. **Key material never leaves the device unencrypted.** Backup keys are encrypted with a user-chosen PIN before upload.

---

## Cryptographic Primitives

| Primitive | Library | Use |
|-----------|---------|-----|
| X25519 | `@noble/curves` (ed25519 module) | X3DH Diffie-Hellman |
| Ed25519 | `@noble/curves` | Identity key signatures, sender key signing |
| HKDF-SHA256 | `@noble/hashes` | Key derivation |
| HMAC-SHA256 | `@noble/hashes` | Symmetric ratchet chain key evolution |
| AES-256-GCM | Web Crypto API | Message encryption |
| Argon2id | `argon2` crate (backend) | Password hashing |
| RSA-2048 | `jsonwebtoken` crate | JWT signing (RS256) |

---

## Direct Messages — X3DH + Symmetric Ratchet

### Key Registration

On registration, each user generates and uploads to the backend:

```
Identity Key (IK)      — Ed25519 long-term key pair
Signed PreKey (SPK)    — X25519 medium-term, signed by IK, rotated monthly
One-Time PreKeys (OPK) — X25519 single-use keys, replenished when ≤ 10 remain
```

The backend stores public keys only. Private keys never leave the device.

### Session Establishment (X3DH)

When Alice sends the first DM to Bob:

```
Alice fetches Bob's prekey bundle (IK_B, SPK_B, sig, OPK_B?)

Alice generates:
  EK_A  — ephemeral X25519 key pair

DH computations:
  DH1 = X25519(IK_A.private, SPK_B.public)
  DH2 = X25519(EK_A.private, IK_B.public)
  DH3 = X25519(EK_A.private, SPK_B.public)
  DH4 = X25519(EK_A.private, OPK_B.public)  [if OPK available]

Master secret = HKDF(DH1 || DH2 || DH3 || DH4)

Root Key, Chain Key derived from master secret
```

The ephemeral public key and OPK ID are attached to the first message header so Bob can reconstruct the same shared secret.

### Symmetric Ratchet

After X3DH, each message advances a HMAC-SHA256 chain:

```
Chain Key  →  HMAC(Chain Key, 0x01)  →  Message Key  →  AES-256-GCM(plaintext)
           →  HMAC(Chain Key, 0x02)  →  next Chain Key
```

Message keys are single-use and derived in order. Out-of-order messages are handled by caching skipped message keys (up to 100 skips).

---

## Group Messages — Sender Keys

Channel messages use a Sender Key scheme (similar to Signal's group messaging):

### Key Generation

When a user first joins or creates a channel:

```
Sender Key = random 32 bytes
Signing Key = Ed25519 key pair (private stays on device)
```

### Distribution

The sender key is distributed to each existing channel member individually, encrypted with their X3DH session key:

```
for each_member in channel_members:
    encrypt(sender_key, x3dh_session_key[each_member])
    POST /api/v1/channels/{id}/sender-key-distribution
```

New members receive pending distributions when they load the channel.

### Message Encryption

```
Chain Key (from Sender Key via HKDF)
  ↓ HKDF iteration per message
Message Key
  ↓ AES-256-GCM
Ciphertext

Wire format: base64(Ed25519_signature_64_bytes || AES_ciphertext)
```

Recipients:
1. Verify Ed25519 signature against sender's cached signing public key
2. Derive message key from their copy of the sender key chain
3. AES-256-GCM decrypt

### Re-keying

The sender key is rotated whenever a member leaves the channel (post-compromise security). `prepareChannel()` is idempotent — if a key already exists it fetches pending distributions instead of generating a new one.

---

## Key Backup

Users can back up their key material encrypted with a PIN:

```
PIN  →  PBKDF2-SHA256 (100k iterations, device salt)  →  Wrap Key
                                                              ↓
Key bundle (IK, ratchet state)  →  AES-256-GCM  →  Encrypted blob
                                                              ↓
                                                    PUT /api/v1/keys/backup
```

The server stores only the encrypted blob and the salt. The PIN and plaintext key material are never transmitted.

---

## Media Encryption

Audio Yaps and Video Clips are encrypted client-side before upload to Cloudflare R2:

```
Random 256-bit content key
  ↓ AES-256-GCM
Encrypted file  →  R2 (opaque blob)

Content key  →  encrypted with recipient's session key
            →  sent via E2EE channel message
```

R2 stores ciphertext only. Even if the R2 bucket were compromised, media content remains unreadable.

---

## Transport Security

- All API traffic over TLS 1.2+ (enforced by Fly.io + Cloudflare)
- WebSocket connections over WSS
- HSTS: `max-age=63072000; includeSubDomains; preload`
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Content-Security-Policy: default-src 'none'; frame-ancestors 'none'` (API responses)

---

## Authentication Security

- **Passwords:** Argon2id with random salt (memory-hard, GPU-resistant)
- **Access tokens:** JWT RS256, 15-minute expiry
- **Refresh tokens:** opaque 256-bit random token, 30-day expiry, stored hashed in DB
- **CSRF:** Double-submit cookie pattern on all state-mutating endpoints (`X-CSRF-Token` header must match `csrf_token` cookie)
- **Rate limiting:** 100 req/min per IP (API); 5 msg/sec per user (WebSocket, burst 20); 10 login attempts/min per IP

---

## Threat Model

| Threat | Mitigation |
|--------|-----------|
| Server compromise (DB leak) | Messages are ciphertext — server never has keys. Passwords are Argon2id hashed. |
| Man-in-the-middle | TLS + key fingerprint pinning (Tauri/Capacitor). Users can verify key fingerprints out-of-band. |
| Stolen device | Keys in IndexedDB (Web Crypto non-extractable where possible). Backup requires PIN. |
| Replay attacks | AES-GCM nonces are unique per message. Ratchet keys are single-use. |
| CSRF | Double-submit cookie on all mutating endpoints. |
| Brute-force auth | Argon2id + IP-level rate limiting on login. |
| Metadata analysis by parent | By design — parents see metadata only, never content. Parental controls are a trust boundary, not a backdoor. |
| Malicious server distributing wrong keys | Key fingerprints are visible in-app. Users should verify out-of-band for sensitive conversations. |

### What Yapper Cannot Protect Against

- **Compromised endpoint:** If a device is fully compromised (keylogger, root access), the attacker can read decrypted messages on-screen.
- **Screenshot/forwarding:** There is no technical prevention for a recipient copying message content.
- **Legal compulsion:** Yapper can produce ciphertext and metadata in response to legal process. It cannot produce plaintext (it doesn't have it).
