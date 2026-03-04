/**
 * Client-side AES-256-GCM encrypt/decrypt for E2EE media blobs (Yaps + Clips).
 *
 * The AES key and IV are generated fresh for every upload, then embedded
 * inside the Signal-encrypted message payload.  The server only ever receives
 * the encrypted blob — it has no access to keys or plaintext.
 *
 * Uses the Web Crypto API, available in all target environments:
 * browser, Tauri webview, and Capacitor WebView.
 */

const ALGO = 'AES-GCM';
const KEY_LENGTH = 256; // bits
const IV_LENGTH = 12;   // bytes (96-bit IV — recommended for AES-GCM)

// ─── Helpers ──────────────────────────────────────────────────────────────────

function bufferToBase64(buf: ArrayBuffer): string {
	return btoa(String.fromCharCode(...new Uint8Array(buf)));
}

function base64ToBuffer(b64: string): ArrayBuffer {
	const binary = atob(b64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes.buffer;
}

// ─── Public API ───────────────────────────────────────────────────────────────

export interface EncryptedMedia {
	/** AES-GCM encrypted bytes ready for upload to R2. */
	encrypted: ArrayBuffer;
	/** Base64-encoded raw AES-256 key. Embed in Signal payload. */
	key: string;
	/** Base64-encoded 96-bit IV. Embed in Signal payload. */
	iv: string;
}

/**
 * Encrypt a media Blob with AES-256-GCM.
 *
 * Generates a fresh random key and IV for every call — two encryptions of
 * identical content will always produce different output.
 */
export async function encryptMedia(blob: Blob): Promise<EncryptedMedia> {
	// Normalize into current-realm bytes to avoid cross-realm BufferSource issues in tests/SSR.
	const plaintextBytes = new Uint8Array(await blob.arrayBuffer());

	const rawKey = crypto.getRandomValues(new Uint8Array(KEY_LENGTH / 8));
	const iv = crypto.getRandomValues(new Uint8Array(IV_LENGTH));

	const cryptoKey = await crypto.subtle.importKey(
		'raw',
		rawKey,
		{ name: ALGO },
		false,             // not extractable (we already hold rawKey)
		['encrypt']
	);

	const encrypted = await crypto.subtle.encrypt(
		{ name: ALGO, iv },
		cryptoKey,
		plaintextBytes
	);

	return {
		encrypted,
		key: bufferToBase64(rawKey.buffer),
		iv: bufferToBase64(iv.buffer)
	};
}

/**
 * Decrypt an AES-256-GCM encrypted ArrayBuffer back into a Blob.
 *
 * @param encrypted  The raw encrypted bytes fetched from R2.
 * @param keyB64     Base64-encoded AES key (from Signal payload).
 * @param ivB64      Base64-encoded IV (from Signal payload).
 * @param mimeType   MIME type to set on the resulting Blob (e.g. 'audio/webm').
 */
export async function decryptMedia(
	encrypted: ArrayBuffer,
	keyB64: string,
	ivB64: string,
	mimeType: string
): Promise<Blob> {
	const encryptedBytes = new Uint8Array(encrypted);
	const rawKey = base64ToBuffer(keyB64);
	const iv = new Uint8Array(base64ToBuffer(ivB64));

	const cryptoKey = await crypto.subtle.importKey(
		'raw',
		rawKey,
		{ name: ALGO },
		false,
		['decrypt']
	);

	const plaintext = await crypto.subtle.decrypt(
		{ name: ALGO, iv },
		cryptoKey,
		encryptedBytes
	);

	return new Blob([plaintext], { type: mimeType });
}
