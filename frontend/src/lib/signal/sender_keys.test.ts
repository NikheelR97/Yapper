import { describe, expect, it } from 'vitest';
import { decryptWithSenderKey, encryptWithSenderKey, generateSenderKey } from './sender_keys.js';
import type { SenderKeyRecord } from './types.js';

describe('sender_keys historical decrypt', () => {
	it('allows historical decrypt without rewinding live ratchet state', async () => {
		const channelId = 'channel-1';
		const senderId = 'user-2';
		const encoder = new TextEncoder();
		const decoder = new TextDecoder();

		let senderKey = generateSenderKey(channelId);
		const seedChainKey = senderKey.chainKey.slice();
		const signingPubKey = senderKey.signingPubKey.slice();

		const encrypted = [];
		for (const text of ['one', 'two', 'three']) {
			const out = await encryptWithSenderKey(senderKey, encoder.encode(text));
			encrypted.push(out.encrypted);
			senderKey = out.updatedKey;
		}

		let record: SenderKeyRecord = {
			channelId,
			senderId,
			chainKey: seedChainKey,
			signingPubKey,
			iteration: 0,
			initialChainKey: seedChainKey,
			initialIteration: 0,
		};

		for (const [idx, expected] of ['one', 'two', 'three'].entries()) {
			const out = await decryptWithSenderKey(record, encrypted[idx]);
			expect(decoder.decode(out.plaintext)).toBe(expected);
			record = out.updatedRecord;
		}

		expect(record.iteration).toBe(3);

		await expect(decryptWithSenderKey(record, encrypted[0])).rejects.toThrow(/already consumed/i);

		const historical = await decryptWithSenderKey(record, encrypted[0], { allowHistorical: true });
		expect(decoder.decode(historical.plaintext)).toBe('one');
		expect(historical.updatedRecord.iteration).toBe(3);
	});
});
