import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

import { toast } from './toast.js';

function clearToasts() {
	for (const entry of get(toast)) {
		toast.remove(entry.id);
	}
}

describe('toast store', () => {
	beforeEach(() => {
		clearToasts();
		vi.useFakeTimers();
		let nextId = 1;
		vi.spyOn(globalThis.crypto, 'randomUUID').mockImplementation(() => `toast-${nextId++}`);
	});

	it('uses default durations for success and warning toasts', () => {
		toast.success('Saved');
		toast.warning('Careful');

		expect(get(toast)).toEqual([
			{ id: 'toast-1', type: 'success', message: 'Saved', duration: 4000 },
			{ id: 'toast-2', type: 'warning', message: 'Careful', duration: 6000 },
		]);

		vi.advanceTimersByTime(4_000);
		expect(get(toast)).toEqual([
			{ id: 'toast-2', type: 'warning', message: 'Careful', duration: 6000 },
		]);

		vi.advanceTimersByTime(2_000);
		expect(get(toast)).toEqual([]);
	});

	it('keeps error toasts until they are manually dismissed', () => {
		const id = toast.error('Something broke');

		vi.advanceTimersByTime(60_000);
		expect(get(toast)).toEqual([
			{ id, type: 'error', message: 'Something broke', duration: 0 },
		]);

		toast.remove(id);
		expect(get(toast)).toEqual([]);
	});

	it('caps visible toasts to the most recent three entries', () => {
		const ids = [
			toast.info('One'),
			toast.info('Two'),
			toast.info('Three'),
			toast.info('Four'),
		];

		expect(get(toast)).toEqual([
			{ id: ids[1], type: 'info', message: 'Two', duration: 4000 },
			{ id: ids[2], type: 'info', message: 'Three', duration: 4000 },
			{ id: ids[3], type: 'info', message: 'Four', duration: 4000 },
		]);
	});

	it('respects custom durations', () => {
		const id = toast.success('Short lived', 250);
		expect(get(toast)).toEqual([
			{ id, type: 'success', message: 'Short lived', duration: 250 },
		]);

		vi.advanceTimersByTime(249);
		expect(get(toast)).toHaveLength(1);

		vi.advanceTimersByTime(1);
		expect(get(toast)).toEqual([]);
	});
});
