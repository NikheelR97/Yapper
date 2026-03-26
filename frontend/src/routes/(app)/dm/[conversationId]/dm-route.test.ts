import { describe, expect, it, vi } from 'vitest';
import { createDmHistoryLoader } from '$stores/conversations.js';

describe('DM route loader', () => {
	it('reloads history when the conversation route changes', async () => {
		const loadMessages = vi.fn().mockResolvedValue(undefined);
		const loader = createDmHistoryLoader(loadMessages);
		const setLoading = vi.fn();
		const setLoadError = vi.fn();

		await loader.requestLoad('conv-1', 'peer-1', setLoading, setLoadError);
		await loader.requestLoad('conv-2', 'peer-2', setLoading, setLoadError);

		expect(loadMessages).toHaveBeenCalledWith('conv-1', 'peer-1');
		expect(loadMessages).toHaveBeenCalledWith('conv-2', 'peer-2');
		expect(loadMessages).toHaveBeenCalledTimes(2);
		expect(setLoading).toHaveBeenCalledWith(true);
		expect(setLoading).toHaveBeenCalledWith(false);
		expect(setLoadError).toHaveBeenCalledWith(false);
	});

	it('does not reload the same conversation twice', async () => {
		const loadMessages = vi.fn().mockResolvedValue(undefined);
		const loader = createDmHistoryLoader(loadMessages);

		await loader.requestLoad('conv-1', 'peer-1', vi.fn(), vi.fn());
		await loader.requestLoad('conv-1', 'peer-1', vi.fn(), vi.fn());

		expect(loadMessages).toHaveBeenCalledTimes(1);
	});
});
