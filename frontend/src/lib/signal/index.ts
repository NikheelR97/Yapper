/**
 * Signal Protocol WASM wrapper.
 * Wraps @signalapp/libsignal-client for use in browser/Tauri/Capacitor webviews.
 *
 * Full implementation: Phase 3 (E2EE core).
 * This file establishes the module shape and WASM initialization pattern.
 */

let initialized = false;

/**
 * Initialize the Signal WASM module.
 * Must be called once on app start (layout.svelte onMount) before any crypto operations.
 */
export async function initSignal(): Promise<void> {
	if (initialized) return;

	// @signalapp/libsignal-client ships a WASM binary that must be initialized
	// before any Signal operations. This takes ~200ms on first call.
	const { initializeFfi } = await import('@signalapp/libsignal-client');

	// The WASM binary path — bundled by Vite
	const wasmPath = new URL(
		'@signalapp/libsignal-client/libsignal_client.wasm',
		import.meta.url
	);

	await initializeFfi(wasmPath.href);
	initialized = true;
}

/**
 * Generate a new identity key pair (Ed25519 + X25519).
 * Called once during account creation/device registration.
 * Private key stored in keystore; public key uploaded to server.
 */
export async function generateIdentityKey() {
	// TODO: Phase 3 — implement with libsignal IdentityKeyPair
	throw new Error('Signal Phase 3: generateIdentityKey not yet implemented');
}

/**
 * Generate a signed prekey for X3DH key exchange.
 * Rotated weekly.
 */
export async function generateSignedPreKey(_identityKey: unknown, _keyId: number) {
	// TODO: Phase 3 — implement with libsignal SignedPreKeyRecord
	throw new Error('Signal Phase 3: generateSignedPreKey not yet implemented');
}

/**
 * Generate a batch of one-time prekeys.
 * @param count Number to generate (default: 100)
 */
export async function generateOneTimePreKeys(_count = 100) {
	// TODO: Phase 3 — implement with libsignal PreKeyRecord
	throw new Error('Signal Phase 3: generateOneTimePreKeys not yet implemented');
}

/**
 * Encrypt a plaintext message using an established Signal session.
 */
export async function encryptMessage(_sessionId: string, _plaintext: Uint8Array) {
	// TODO: Phase 3 — SessionCipher.encrypt
	throw new Error('Signal Phase 3: encryptMessage not yet implemented');
}

/**
 * Decrypt a ciphertext using an established Signal session.
 */
export async function decryptMessage(_sessionId: string, _ciphertext: Uint8Array) {
	// TODO: Phase 3 — SessionCipher.decrypt
	throw new Error('Signal Phase 3: decryptMessage not yet implemented');
}
