/**
 * PIN-based Signal keystore backup / restore.
 *
 * Encryption scheme (all client-side, server never sees the PIN or plaintext):
 *   1. Derive a 256-bit AES-GCM key from the PIN using PBKDF2-SHA-256
 *      with a random 16-byte salt and 600 000 iterations.
 *   2. Encrypt the JSON-serialised keystore snapshot with AES-256-GCM
 *      using a random 12-byte IV.
 *   3. Concatenate salt(16) || iv(12) || ciphertext+tag and base64-encode it.
 *   4. Upload the blob to PUT /api/v2/keys/backup.
 *
 * Restore reverses the process: fetch blob → split → derive key → decrypt.
 *
 * Uint8Array values are serialised as base64 strings inside the JSON snapshot
 * so the blob stays ASCII-safe and compact.
 */

import { api } from '$lib/api/client.js';
import { exportSignalSnapshot, importSignalSnapshot } from './keystore.js';
const PBKDF2_ITERS = 600_000;

// ─── Helpers ──────────────────────────────────────────────────────────────────

function u8ToB64(u8: Uint8Array): string {
	return btoa(String.fromCharCode(...u8));
}

function b64ToU8(b64: string): Uint8Array {
	return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

function jsonReplacer(_key: string, value: unknown): unknown {
	if (value instanceof Uint8Array) return { __u8: u8ToB64(value) };
	return value;
}

function jsonReviver(_key: string, value: unknown): unknown {
	if (value && typeof value === 'object' && '__u8' in (value as object)) {
		return b64ToU8((value as { __u8: string }).__u8);
	}
	return value;
}

async function deriveKey(pin: string, salt: Uint8Array): Promise<CryptoKey> {
	const enc = new TextEncoder();
	const keyMaterial = await crypto.subtle.importKey('raw', enc.encode(pin), 'PBKDF2', false, [
		'deriveKey'
	]);
	return crypto.subtle.deriveKey(
		{ name: 'PBKDF2', salt: salt.slice(), hash: 'SHA-256', iterations: PBKDF2_ITERS },
		keyMaterial,
		{ name: 'AES-GCM', length: 256 },
		false,
		['encrypt', 'decrypt']
	);
}

// ─── Snapshot ─────────────────────────────────────────────────────────────────

async function exportKeystore(): Promise<string> {
	return JSON.stringify(await exportSignalSnapshot(), jsonReplacer);
}

async function importKeystore(snapshot: string): Promise<void> {
	const data = JSON.parse(snapshot, jsonReviver) as Parameters<typeof importSignalSnapshot>[0];
	await importSignalSnapshot(data);
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Encrypt the local Signal keystore with `pin` and upload to the server.
 * Call after any key rotation or session establishment.
 */
export async function backupKeys(pin: string): Promise<void> {
	const snapshot = await exportKeystore();
	const plaintext = new TextEncoder().encode(snapshot);

	const salt = crypto.getRandomValues(new Uint8Array(16));
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const key = await deriveKey(pin, salt);

	const ciphertextBuf = await crypto.subtle.encrypt(
		{ name: 'AES-GCM', iv: iv.slice() },
		key,
		plaintext.slice()
	);
	const ciphertext = new Uint8Array(ciphertextBuf);

	// Concatenate salt(16) || iv(12) || ciphertext+tag
	const blob = new Uint8Array(16 + 12 + ciphertext.byteLength);
	blob.set(salt, 0);
	blob.set(iv, 16);
	blob.set(ciphertext, 28);

	await api.put('/api/v2/keys/backup', { encrypted_blob: u8ToB64(blob) });
}

/**
 * Fetch the encrypted backup from the server, decrypt with `pin`, and
 * restore all Signal keys into the local Signal store.
 *
 * Returns `true` on success, `false` if no backup exists on the server.
 * Throws on decryption failure (wrong PIN).
 */
export async function restoreKeys(pin: string, sourceDeviceId?: string): Promise<boolean> {
	let resp: { encrypted_blob: string };
	try {
		const query = sourceDeviceId
			? `/api/v2/keys/backup?source_device_id=${encodeURIComponent(sourceDeviceId)}`
			: '/api/v2/keys/backup';
		resp = await api.get<{ encrypted_blob: string }>(query);
	} catch (err: unknown) {
		// 404 = no backup stored yet
		if (err && typeof err === 'object' && 'status' in err && (err as { status: number }).status === 404) {
			return false;
		}
		throw err;
	}

	const blob = b64ToU8(resp.encrypted_blob);
	if (blob.length < 45) throw new Error('Backup blob is too short');

	const salt = blob.slice(0, 16);
	const iv = blob.slice(16, 28);
	const ciphertext = blob.slice(28);

	const key = await deriveKey(pin, salt);

	let plaintextBuf: ArrayBuffer;
	try {
		plaintextBuf = await crypto.subtle.decrypt(
			{ name: 'AES-GCM', iv: iv.slice() },
			key,
			ciphertext.slice()
		);
	} catch {
		throw new Error('Decryption failed — check your PIN');
	}

	const snapshot = new TextDecoder().decode(plaintextBuf);
	await importKeystore(snapshot);
	if (sourceDeviceId) {
		await api.post('/api/v2/keys/backup/restore', {
			source_device_id: sourceDeviceId,
		});
	}
	return true;
}
