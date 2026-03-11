/**
 * IndexedDB encryption at rest (M-05).
 *
 * Encrypts sensitive Signal Protocol key material in IndexedDB using
 * AES-256-GCM. The encryption key is stored in localStorage (persists
 * across browser sessions). Desktop platforms use Stronghold vault
 * instead — this module is unused there.
 */

let _cachedKey: CryptoKey | null = null;
let _scopeKey: string | null = null;

function storageKeyName(scope: string): string {
	return `yapper-enc-${scope}`;
}

function bytesToB64(bytes: Uint8Array): string {
	let binary = '';
	for (let i = 0; i < bytes.length; i++) {
		binary += String.fromCharCode(bytes[i]);
	}
	return btoa(binary);
}

function b64ToBytes(b64: string): Uint8Array {
	const binary = atob(b64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

function jsonReplacer(_key: string, value: unknown): unknown {
	if (value instanceof Uint8Array) {
		return { __t: 'u8', __d: bytesToB64(value) };
	}
	return value;
}

function jsonReviver(_key: string, value: unknown): unknown {
	if (
		value &&
		typeof value === 'object' &&
		(value as Record<string, unknown>).__t === 'u8' &&
		typeof (value as Record<string, unknown>).__d === 'string'
	) {
		return b64ToBytes((value as Record<string, string>).__d);
	}
	return value;
}

export interface EncryptedEnvelope {
	__yenc: 1;
	iv: string;
	ct: string;
	[key: string]: unknown;
}

function isEncryptedEnvelope(value: unknown): value is EncryptedEnvelope {
	return (
		value !== null &&
		typeof value === 'object' &&
		(value as Record<string, unknown>).__yenc === 1
	);
}

async function getOrCreateKey(): Promise<CryptoKey> {
	if (_cachedKey) return _cachedKey;
	if (!_scopeKey) {
		throw new Error('IDB encryption not initialized');
	}

	const stored = localStorage.getItem(storageKeyName(_scopeKey));
	if (stored) {
		const raw = b64ToBytes(stored);
		_cachedKey = await crypto.subtle.importKey(
			'raw',
			raw.slice(),
			'AES-GCM',
			true,
			['encrypt', 'decrypt'],
		);
		return _cachedKey;
	}

	_cachedKey = await crypto.subtle.generateKey(
		{ name: 'AES-GCM', length: 256 },
		true,
		['encrypt', 'decrypt'],
	);
	const exported = await crypto.subtle.exportKey('raw', _cachedKey);
	localStorage.setItem(
		storageKeyName(_scopeKey),
		bytesToB64(new Uint8Array(exported)),
	);
	return _cachedKey;
}

export async function initIdbEncryption(scopeKey: string): Promise<void> {
	try {
		if (typeof localStorage === 'undefined' || typeof crypto?.subtle === 'undefined') {
			return;
		}
		_scopeKey = scopeKey;
		_cachedKey = null;
		await getOrCreateKey();
	} catch {
		// Encryption not available in this environment — proceed unencrypted
		_cachedKey = null;
		_scopeKey = null;
	}
}

export function clearIdbEncryptionKey(removeFromStorage = false): void {
	if (removeFromStorage && _scopeKey) {
		try {
			localStorage.removeItem(storageKeyName(_scopeKey));
		} catch {
			// ignore
		}
	}
	_cachedKey = null;
	_scopeKey = null;
}

export function isIdbEncryptionReady(): boolean {
	return _cachedKey !== null && _scopeKey !== null;
}

/**
 * Encrypt a value for IndexedDB storage.
 * If keyPathField is provided, its value is preserved unencrypted so
 * IndexedDB can still use it as the object store key.
 */
export async function idbEncryptValue(
	value: unknown,
	keyPathField?: string,
): Promise<EncryptedEnvelope> {
	const key = await getOrCreateKey();
	const plaintext = new TextEncoder().encode(
		JSON.stringify(value, jsonReplacer),
	);
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const ct = await crypto.subtle.encrypt(
		{ name: 'AES-GCM', iv },
		key,
		plaintext,
	);

	const envelope: EncryptedEnvelope = {
		__yenc: 1,
		iv: bytesToB64(iv),
		ct: bytesToB64(new Uint8Array(ct)),
	};

	if (keyPathField && value && typeof value === 'object') {
		envelope[keyPathField] = (value as Record<string, unknown>)[keyPathField];
	}

	return envelope;
}

/**
 * Decrypt a value from IndexedDB.
 * Returns the value as-is if it's not an encrypted envelope (backwards compat).
 */
export async function idbDecryptValue<T>(value: unknown): Promise<T> {
	if (!isEncryptedEnvelope(value)) {
		return value as T;
	}

	const key = await getOrCreateKey();
	const iv = b64ToBytes(value.iv);
	const ct = b64ToBytes(value.ct);
	const plaintext = await crypto.subtle.decrypt(
		{ name: 'AES-GCM', iv: iv.slice() },
		key,
		ct.slice(),
	);

	return JSON.parse(
		new TextDecoder().decode(plaintext),
		jsonReviver,
	) as T;
}
