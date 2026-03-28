const EXPLORE_BASE_PATH = '/api/v2/explore/';
const EXPLORE_BASE_ROUTE = '/api/v2/explore';

const CACHEABLE_PATHS = new Set(['trending-tags', 'communities', 'live-servers']);

export function getExploreSubPath(pathname: string): string {
	if (pathname === EXPLORE_BASE_ROUTE) {
		return '';
	}
	if (pathname.startsWith(EXPLORE_BASE_PATH)) {
		return pathname.slice(EXPLORE_BASE_PATH.length).split('/')[0];
	}
	if (pathname.startsWith(`${EXPLORE_BASE_ROUTE}/`)) {
		return pathname.slice(EXPLORE_BASE_ROUTE.length + 1).split('/')[0];
	}
	return pathname.split('/')[0];
}

export function isCacheableExplorePath(subPath: string): boolean {
	return CACHEABLE_PATHS.has(subPath);
}

export function buildOriginUrl(origin: string, pathname: string, search: string): string {
	return `${origin}${pathname}${search}`;
}
