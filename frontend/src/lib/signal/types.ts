/**
 * Shared Signal Protocol types.
 * Server only stores public keys and ciphertext — private keys NEVER leave the device.
 */

export interface IdentityKeyPair {
	/** X25519 public key (32 bytes) — used for DH in X3DH. */
	dhPublicKey: Uint8Array;
	/** X25519 private key (32 bytes) — stored locally only. */
	dhPrivateKey: Uint8Array;
	/** Ed25519 public key (32 bytes) — used to verify signed prekeys. */
	sigPublicKey: Uint8Array;
	/** Ed25519 private key (32 bytes seed) — stored locally only. */
	sigPrivateKey: Uint8Array;
}

export interface PreKeyPair {
	keyId: number;
	/** X25519 public key (32 bytes). */
	publicKey: Uint8Array;
	/** X25519 private key (32 bytes) — stored locally only. */
	privateKey: Uint8Array;
}

export interface SignedPreKey extends PreKeyPair {
	/** Ed25519 signature over publicKey (64 bytes). */
	signature: Uint8Array;
	createdAt: number;
}

export interface Session {
	/** UUID of the DM conversation this session belongs to. */
	conversationId: string;
	/** The other party's user UUID. */
	peerId: string;
	rootKey: Uint8Array;
	sendChainKey: Uint8Array;
	receiveChainKey: Uint8Array | null;
	sendMsgNum: number;
	receiveMsgNum: number;
}

/** Key bundle fetched from the server for a target user. */
export interface KeyBundle {
	userId: string;
	deviceId: number;
	/** Base64-encoded X25519 DH public key. */
	identity_dh_key: string;
	/** Base64-encoded Ed25519 signing public key. */
	identity_sig_key: string;
	signed_prekey_id: number;
	/** Base64-encoded X25519 signed prekey public key. */
	signed_prekey: string;
	/** Base64-encoded Ed25519 signature. */
	signed_prekey_sig: string;
	one_time_prekey_id: number | null;
	/** Base64-encoded X25519 one-time prekey public key (may be absent). */
	one_time_prekey: string | null;
}

export interface EncryptedMessage {
	/** Base64-encoded AES-256-GCM ciphertext (12-byte IV prepended). */
	ciphertext: string;
	/** Base64-encoded X25519 ephemeral public key — only on first message (X3DH). */
	ephemeralKey?: string;
	/** OPK id used for X3DH — only on first message. */
	opkId?: number;
	msgNum: number;
}
