import { platform as runtimePlatform } from '$lib/plugins/tauri-compat.js';
import type { AuthDevice } from '$stores/auth.js';

const INSTALLATION_ID_KEY = 'yapper_installation_id';

export interface DeviceBootstrap {
	installation_id: string;
	platform: 'web' | 'tauri' | 'capacitor';
	label: string;
}

interface ServerDeviceShape {
	id: string;
	signal_device_id: number;
	installation_id: string | null;
	platform: 'web' | 'tauri' | 'capacitor';
	label: string;
	trust_state: 'trusted' | 'pending_trust' | 'revoked';
	created_at: string;
	last_seen_at: string | null;
	approved_at: string | null;
	revoked_at: string | null;
}

export function getDeviceBootstrap(): DeviceBootstrap {
	const installationId = getOrCreateInstallationId();
	const platform = runtimePlatform();
	return {
		installation_id: installationId,
		platform,
		label: defaultDeviceLabel(platform),
	};
}

export function normalizeServerDevice(device: ServerDeviceShape): AuthDevice {
	return {
		id: device.id,
		signalDeviceId: device.signal_device_id,
		installationId: device.installation_id,
		platform: device.platform,
		label: device.label,
		trustState: device.trust_state,
		createdAt: device.created_at,
		lastSeenAt: device.last_seen_at,
		approvedAt: device.approved_at,
		revokedAt: device.revoked_at,
	};
}

function getOrCreateInstallationId(): string {
	if (typeof window === 'undefined') {
		return crypto.randomUUID();
	}

	let existing = window.localStorage.getItem(INSTALLATION_ID_KEY);
	if (existing) {
		return existing;
	}

	existing = crypto.randomUUID();
	window.localStorage.setItem(INSTALLATION_ID_KEY, existing);
	return existing;
}

function defaultDeviceLabel(platform: DeviceBootstrap['platform']): string {
	switch (platform) {
		case 'tauri':
			return 'Desktop App';
		case 'capacitor':
			return 'Mobile App';
		default:
			return 'Web Browser';
	}
}
