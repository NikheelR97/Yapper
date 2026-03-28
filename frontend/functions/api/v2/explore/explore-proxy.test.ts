import { describe, expect, it } from 'vitest';
import {
	buildOriginUrl,
	getExploreSubPath,
	isCacheableExplorePath,
} from './explore-proxy';

describe('explore edge proxy helpers', () => {
	it('extracts the versioned explore sub-path from v2 routes', () => {
		expect(getExploreSubPath('/api/v2/explore')).toBe('');
		expect(getExploreSubPath('/api/v2/explore/trending-tags')).toBe('trending-tags');
		expect(getExploreSubPath('/api/v2/explore/communities')).toBe('communities');
		expect(getExploreSubPath('/api/v2/explore/search')).toBe('search');
	});

	it('only caches the explicitly allowed explore endpoints', () => {
		expect(isCacheableExplorePath('trending-tags')).toBe(true);
		expect(isCacheableExplorePath('communities')).toBe(true);
		expect(isCacheableExplorePath('live-servers')).toBe(true);
		expect(isCacheableExplorePath('search')).toBe(false);
		expect(isCacheableExplorePath('top-yappers')).toBe(false);
	});

	it('builds origin urls without changing path or query', () => {
		expect(buildOriginUrl('https://yapper-api.fly.dev', '/api/v2/explore/search', '?q=test')).toBe(
			'https://yapper-api.fly.dev/api/v2/explore/search?q=test',
		);
	});
});
