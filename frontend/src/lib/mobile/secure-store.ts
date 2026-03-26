import { isCapacitor } from '$lib/plugins/tauri-compat.js';

const MISSING_ITEM_ERROR = 'Item with given key does not exist';

async function loadSecureStoragePlugin() {
	const { SecureStoragePlugin } = await import('capacitor-secure-storage-plugin');
	return SecureStoragePlugin;
}

export async function getMobileSecureItem(key: string): Promise<string | null> {
	if (!isCapacitor()) {
		return null;
	}

	try {
		const plugin = await loadSecureStoragePlugin();
		const result = await plugin.get({ key });
		return result.value;
	} catch (err) {
		const message = err instanceof Error ? err.message : String(err);
		if (message.includes(MISSING_ITEM_ERROR)) {
			return null;
		}
		throw err;
	}
}

export async function setMobileSecureItem(key: string, value: string): Promise<void> {
	if (!isCapacitor()) {
		return;
	}

	const plugin = await loadSecureStoragePlugin();
	await plugin.set({ key, value });
}

export async function removeMobileSecureItem(key: string): Promise<void> {
	if (!isCapacitor()) {
		return;
	}

	try {
		const plugin = await loadSecureStoragePlugin();
		await plugin.remove({ key });
	} catch (err) {
		const message = err instanceof Error ? err.message : String(err);
		if (message.includes(MISSING_ITEM_ERROR)) {
			return;
		}
		throw err;
	}
}
