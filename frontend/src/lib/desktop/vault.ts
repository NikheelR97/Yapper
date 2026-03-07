import { isTauri } from '$lib/plugins/tauri-compat.js';

const STRONGHOLD_FILE = 'signal-vault.hold';
const STRONGHOLD_CLIENT = 'signal-vault';
const SIGNAL_RECORD_PREFIX = 'signal-secrets:';

type StrongholdModule = typeof import('@tauri-apps/plugin-stronghold');
type StrongholdInstance = import('@tauri-apps/plugin-stronghold').Stronghold;
type StoreInstance = import('@tauri-apps/plugin-stronghold').Store;

interface VaultHandle {
	stronghold: StrongholdInstance;
	store: StoreInstance;
}

let vaultHandle: VaultHandle | null = null;

function u8ToB64(u8: Uint8Array): string {
	return btoa(String.fromCharCode(...u8));
}

function b64ToU8(b64: string): Uint8Array {
	return Uint8Array.from(atob(b64), (char) => char.charCodeAt(0));
}

function jsonReplacer(_key: string, value: unknown): unknown {
	if (value instanceof Uint8Array) {
		return { __u8: u8ToB64(value) };
	}
	return value;
}

function jsonReviver(_key: string, value: unknown): unknown {
	if (value && typeof value === 'object' && '__u8' in (value as object)) {
		return b64ToU8((value as { __u8: string }).__u8);
	}
	return value;
}

async function loadStrongholdModule(): Promise<StrongholdModule> {
	return import('@tauri-apps/plugin-stronghold');
}

async function resolveStrongholdPath(): Promise<string> {
	const { appDataDir, join } = await import('@tauri-apps/api/path');
	return join(await appDataDir(), STRONGHOLD_FILE);
}

function requireDesktopVault(): VaultHandle {
	if (!vaultHandle) {
		throw new Error('Desktop vault is locked');
	}
	return vaultHandle;
}

function recordKey(scopeKey: string): string {
	return `${SIGNAL_RECORD_PREFIX}${scopeKey}`;
}

export function desktopVaultSupported(): boolean {
	return isTauri();
}

export function isDesktopVaultUnlocked(): boolean {
	return vaultHandle != null;
}

export async function desktopSignalVaultExists(): Promise<boolean> {
	if (!desktopVaultSupported()) {
		return false;
	}

	const { invoke } = await import('@tauri-apps/api/core');
	return invoke<boolean>('stronghold_exists');
}

export async function unlockDesktopVault(passphrase: string): Promise<void> {
	if (!desktopVaultSupported()) {
		return;
	}
	if (!passphrase.trim()) {
		throw new Error('Vault passphrase is required');
	}

	const [{ Stronghold }, strongholdPath] = await Promise.all([
		loadStrongholdModule(),
		resolveStrongholdPath(),
	]);

	let stronghold: StrongholdInstance;
	try {
		stronghold = await Stronghold.load(strongholdPath, passphrase);
	} catch {
		throw new Error('Unable to unlock secure vault');
	}

	let client;
	try {
		client = await stronghold.loadClient(STRONGHOLD_CLIENT);
	} catch {
		client = await stronghold.createClient(STRONGHOLD_CLIENT);
		await stronghold.save();
	}

	vaultHandle = {
		stronghold,
		store: client.getStore(),
	};
}

export async function lockDesktopVault(): Promise<void> {
	if (!vaultHandle) {
		return;
	}

	const { stronghold } = vaultHandle;
	vaultHandle = null;
	await stronghold.unload().catch(() => {});
}

export async function loadDesktopSignalVaultRecord<T>(scopeKey: string): Promise<T | null> {
	if (!desktopVaultSupported()) {
		return null;
	}

	const { store } = requireDesktopVault();
	const bytes = await store.get(recordKey(scopeKey));
	if (!bytes) {
		return null;
	}

	const raw = new TextDecoder().decode(new Uint8Array(bytes));
	return JSON.parse(raw, jsonReviver) as T;
}

export async function saveDesktopSignalVaultRecord(
	scopeKey: string,
	value: unknown,
): Promise<void> {
	if (!desktopVaultSupported()) {
		return;
	}

	const { stronghold, store } = requireDesktopVault();
	const encoded = new TextEncoder().encode(JSON.stringify(value, jsonReplacer));
	await store.insert(recordKey(scopeKey), Array.from(encoded));
	await stronghold.save();
}

export async function clearDesktopSignalVaultRecord(scopeKey: string): Promise<void> {
	if (!desktopVaultSupported()) {
		return;
	}

	const { stronghold, store } = requireDesktopVault();
	await store.remove(recordKey(scopeKey));
	await stronghold.save();
}
