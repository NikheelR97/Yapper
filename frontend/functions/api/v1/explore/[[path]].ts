/**
 * Cloudflare Pages Function — edge cache proxy for read-only explore endpoints.
 *
 * Caches responses from the Fly.io backend (jnb) at the edge for 5 minutes,
 * reducing latency for global users from ~150-300ms (origin RTT) to <50ms (edge hit).
 *
 * Cached endpoints:
 *   GET /api/v1/explore/trending-tags
 *   GET /api/v1/explore/communities
 *   GET /api/v1/explore/live-servers
 *
 * Non-GET requests and search queries (which are user-specific) are passed through.
 */

const ORIGIN = 'https://yapper-api.fly.dev';
const CACHE_TTL = 300; // 5 minutes — matches backend trending tags cache

// Only cache these specific sub-paths (no user-specific data)
const CACHEABLE_PATHS = new Set([
	'trending-tags',
	'communities',
	'live-servers',
]);

export const onRequest: PagesFunction = async (context) => {
	const { request } = context;

	// Only cache GET requests
	if (request.method !== 'GET') {
		return fetch(`${ORIGIN}${new URL(request.url).pathname}`, request);
	}

	const url = new URL(request.url);
	const subPath = url.pathname.replace('/api/v1/explore/', '').split('/')[0];

	// Only cache known safe endpoints — search and top-yappers may vary per user
	if (!CACHEABLE_PATHS.has(subPath)) {
		return fetch(`${ORIGIN}${url.pathname}${url.search}`, {
			headers: request.headers,
		});
	}

	// Try edge cache first
	const cache = caches.default;
	const cacheKey = new Request(url.toString(), { method: 'GET' });
	const cached = await cache.match(cacheKey);
	if (cached) return cached;

	// Cache miss — fetch from origin
	const originResponse = await fetch(`${ORIGIN}${url.pathname}${url.search}`, {
		headers: {
			'Accept': 'application/json',
		},
	});

	if (!originResponse.ok) {
		return originResponse;
	}

	// Clone and add cache headers
	const response = new Response(originResponse.body, {
		status: originResponse.status,
		headers: new Headers(originResponse.headers),
	});
	response.headers.set('Cache-Control', `public, max-age=${CACHE_TTL}`);
	response.headers.set('X-Cache-Status', 'MISS');

	// Store in edge cache (non-blocking)
	context.waitUntil(cache.put(cacheKey, response.clone()));

	return response;
};
