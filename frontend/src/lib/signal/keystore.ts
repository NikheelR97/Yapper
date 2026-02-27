/**
 * Abstract key storage layer.
 *
 * Platform routing:
 *   - Desktop (Tauri)   → Tauri Stronghold plugin (hardware-backed on supported OSes)
 *   - Mobile (Capacitor)→ capacitor-secure-storage-plugin (iOS Keychain / Android Keystore)
 *   - Web PWA           → IndexedDB (idb) — acceptable for web-only; private key never leaves client
 *
 * Full implementation: Phase 3.
 */

export interface KeyRecord {
	keyId: number;
	publicKey: Uint8Array;
	privateKey: Uint8Array;
}

/** Detect current platform */
function platform(): 'tauri' | 'capacitor' | 'web' {
	if (typeof window !== 'undefined' && '__TAURI__' in window) return 'tauri';
	// Capacitor sets window.Capacitor
	if (typeof window !== 'undefined' && 'Capacitor' in window) return 'capacitor';
	return 'web';
}

/**
 * Store a private key material.
 * key: namespaced identifier, e.g. 'identity_private', 'signed_prekey_1'
 */
export async function storeKey(_key: string, _material: Uint8Array): Promise<void> {
	const p = platform();
	// TODO: Phase 3 — route to Tauri Stronghold / Capacitor SecureStorage / IDB
	throw new Error(`Phase 3: storeKey not yet implemented for platform=${p}`);
}

/**
 * Retrieve stored key material.
 */
export async function loadKey(_key: string): Promise<Uint8Array | null> {
	const p = platform();
	// TODO: Phase 3 — route to Tauri Stronghold / Capacitor SecureStorage / IDB
	throw new Error(`Phase 3: loadKey not yet implemented for platform=${p}`);
}

/**
 * Delete a stored key (e.g. consumed OPK).
 */
export async function deleteKey(_key: string): Promise<void> {
	const p = platform();
	// TODO: Phase 3 — route to Tauri Stronghold / Capacitor SecureStorage / IDB
	throw new Error(`Phase 3: deleteKey not yet implemented for platform=${p}`);
}
