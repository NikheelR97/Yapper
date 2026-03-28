import { describe, expect, it } from 'vitest';

import { readReceiptsEnabled } from './readReceiptMode.js';

describe('readReceiptsEnabled', () => {
	it('enables read receipts for channel timelines only', () => {
		expect(readReceiptsEnabled('channel')).toBe(true);
		expect(readReceiptsEnabled('dm')).toBe(false);
	});
});
