import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
	id: string;
	type: ToastType;
	message: string;
	duration: number; // ms; 0 = manual dismiss only
}

function createToastStore() {
	const { subscribe, update } = writable<Toast[]>([]);

	function add(type: ToastType, message: string, duration?: number): string {
		const id = crypto.randomUUID();
		const defaultDuration =
			type === 'error' ? 0 : type === 'warning' ? 6000 : 4000;
		const d = duration ?? defaultDuration;

		update((toasts) => {
			const next = [...toasts, { id, type, message, duration: d }];
			// Cap at 3 visible toasts
			return next.length > 3 ? next.slice(next.length - 3) : next;
		});

		if (d > 0) {
			setTimeout(() => remove(id), d);
		}

		return id;
	}

	function remove(id: string) {
		update((toasts) => toasts.filter((t) => t.id !== id));
	}

	return {
		subscribe,
		success: (msg: string, duration?: number) => add('success', msg, duration),
		error: (msg: string, duration?: number) => add('error', msg, duration),
		warning: (msg: string, duration?: number) => add('warning', msg, duration),
		info: (msg: string, duration?: number) => add('info', msg, duration),
		remove,
	};
}

export const toast = createToastStore();
