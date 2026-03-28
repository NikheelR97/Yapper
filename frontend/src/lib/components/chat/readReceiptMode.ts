export type ReadReceiptMode = 'dm' | 'channel';

export function readReceiptsEnabled(mode: ReadReceiptMode): boolean {
	return mode === 'channel';
}
