import { Capacitor, registerPlugin } from '@capacitor/core';
import { api } from '$api/client.js';
import { platform as runtimePlatform } from '$plugins/tauri-compat.js';
import {
	incrementUsage,
	ScreenTimeWebPlugin,
	type ScreenTimeAppUsage,
} from '$plugins/screentime.web.js';

export type ScreenTimeReportPayload = {
	recordedDate: string;
	platform: 'ios' | 'android' | 'web' | 'desktop';
	apps: ScreenTimeAppUsage[];
};

type NativeScreenTimePlugin = {
	collectDailyUsage(): Promise<{ apps: ScreenTimeAppUsage[] }>;
};

const ScreenTime = registerPlugin<NativeScreenTimePlugin>('ScreenTime', {
	web: async () => new ScreenTimeWebPlugin(),
});

function todayIsoDate(): string {
	return new Date().toISOString().slice(0, 10);
}

function resolvePlatform(): 'ios' | 'android' | 'web' | 'desktop' {
	const rt = runtimePlatform();
	if (rt === 'tauri') return 'desktop';
	if (rt === 'capacitor') {
		return Capacitor.getPlatform() === 'ios' ? 'ios' : 'android';
	}
	return 'web';
}

function normalizeApps(apps: ScreenTimeAppUsage[]): ScreenTimeAppUsage[] {
	return apps
		.map((entry) => ({
			appName: entry.appName?.trim() ?? '',
			durationSeconds: Math.max(0, Math.floor(entry.durationSeconds ?? 0)),
		}))
		.filter((entry) => entry.appName.length > 0 && entry.durationSeconds > 0)
		.slice(0, 64);
}

async function collectApps(): Promise<ScreenTimeAppUsage[]> {
	try {
		const result = await ScreenTime.collectDailyUsage();
		return normalizeApps(result.apps ?? []);
	} catch {
		const fallback = new ScreenTimeWebPlugin();
		const result = await fallback.collectDailyUsage();
		return normalizeApps(result.apps ?? []);
	}
}

export async function buildScreenTimeReport(): Promise<ScreenTimeReportPayload | null> {
	const apps = await collectApps();
	if (apps.length === 0) {
		return null;
	}

	return {
		recordedDate: todayIsoDate(),
		platform: resolvePlatform(),
		apps,
	};
}

export async function reportScreenTimeUsage(): Promise<void> {
	const payload = await buildScreenTimeReport();
	if (!payload) {
		return;
	}

	await api.post('/api/v2/screentime/report', payload);
}

export function startYapperUsageTracker(): () => void {
	let lastTick = Date.now();

	const tick = () => {
		const now = Date.now();
		const seconds = (now - lastTick) / 1000;
		lastTick = now;

		if (typeof document !== 'undefined' && document.visibilityState === 'visible') {
			incrementUsage('Yapper', seconds);
		}
	};

	const intervalId = window.setInterval(tick, 30_000);
	const visibilityHandler = () => {
		tick();
	};

	document.addEventListener('visibilitychange', visibilityHandler);
	window.addEventListener('beforeunload', visibilityHandler);

	return () => {
		window.clearInterval(intervalId);
		document.removeEventListener('visibilitychange', visibilityHandler);
		window.removeEventListener('beforeunload', visibilityHandler);
		tick();
	};
}
