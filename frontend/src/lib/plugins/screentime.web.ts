import { WebPlugin } from '@capacitor/core';

const STORAGE_KEY = 'yapper:screentime:v1';
const MAX_SECONDS_PER_APP_PER_DAY = 86_400;

type StoredUsage = {
	date: string;
	apps: Record<string, number>;
};

export type ScreenTimeAppUsage = {
	appName: string;
	durationSeconds: number;
};

function todayIsoDate(): string {
	return new Date().toISOString().slice(0, 10);
}

function loadUsage(): StoredUsage {
	if (typeof window === 'undefined') {
		return { date: todayIsoDate(), apps: {} };
	}

	const raw = window.localStorage.getItem(STORAGE_KEY);
	if (!raw) {
		return { date: todayIsoDate(), apps: {} };
	}

	try {
		const parsed = JSON.parse(raw) as StoredUsage;
		if (!parsed || typeof parsed.date !== 'string' || typeof parsed.apps !== 'object') {
			return { date: todayIsoDate(), apps: {} };
		}
		if (parsed.date !== todayIsoDate()) {
			return { date: todayIsoDate(), apps: {} };
		}
		return parsed;
	} catch {
		return { date: todayIsoDate(), apps: {} };
	}
}

function saveUsage(usage: StoredUsage): void {
	if (typeof window === 'undefined') {
		return;
	}
	window.localStorage.setItem(STORAGE_KEY, JSON.stringify(usage));
}

export function incrementUsage(appName: string, seconds: number): void {
	if (typeof window === 'undefined' || !Number.isFinite(seconds) || seconds <= 0) {
		return;
	}

	const key = appName.trim();
	if (!key) {
		return;
	}

	const usage = loadUsage();
	const current = usage.apps[key] ?? 0;
	usage.apps[key] = Math.min(MAX_SECONDS_PER_APP_PER_DAY, current + Math.floor(seconds));
	saveUsage(usage);
}

export class ScreenTimeWebPlugin extends WebPlugin {
	async collectDailyUsage(): Promise<{ apps: ScreenTimeAppUsage[] }> {
		const usage = loadUsage();
		const apps = Object.entries(usage.apps)
			.map(([appName, durationSeconds]) => ({
				appName,
				durationSeconds,
			}))
			.filter((entry) => entry.durationSeconds > 0)
			.sort((a, b) => b.durationSeconds - a.durationSeconds);

		return { apps };
	}
}
