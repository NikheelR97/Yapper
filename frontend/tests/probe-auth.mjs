const API_URL = process.env.VITE_API_URL ?? 'https://api.yapperhq.com';
const EMAIL = process.env.E2E_PROBE_EMAIL ?? process.env.E2E_EMAIL;
const PASSWORD = process.env.E2E_PROBE_PASSWORD ?? process.env.E2E_PASSWORD;
const INSTALLATION_ID =
	process.env.E2E_PROBE_INSTALLATION_ID ??
	process.env.E2E_PRIMARY_INSTALLATION_ID ??
	'11111111-1111-4111-8111-111111111111';
const LABEL = process.env.E2E_PROBE_LABEL ?? 'GitHub Actions Probe';
const LOGIN_TIMEOUT_MS = Number(process.env.E2E_AUTH_TIMEOUT_MS ?? 15_000);

function isCloudflareBody(bodyText) {
	const body = (bodyText ?? '').toLowerCase();
	return (
		body.includes('just a moment') ||
		body.includes('__cf_chl') ||
		body.includes('cloudflare') ||
		body.includes('attention required')
	);
}

function isNetworkError(error) {
	return (
		error?.name === 'TimeoutError' ||
		error?.name === 'AbortError' ||
		error?.name === 'TypeError'
	);
}

async function main() {
	if (!EMAIL || !PASSWORD) {
		console.error('E2E auth probe requires probe credentials.');
		process.exit(2);
	}

	try {
		const response = await fetch(`${API_URL}/api/v2/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			signal: AbortSignal.timeout(LOGIN_TIMEOUT_MS),
			body: JSON.stringify({
				email: EMAIL,
				password: PASSWORD,
				device: {
					installation_id: INSTALLATION_ID,
					platform: 'web',
					label: LABEL,
				},
			}),
		});

		const contentType = response.headers.get('content-type') ?? '';
		const bodyText = await response.text();
		if (
			(response.status === 403 && isCloudflareBody(bodyText)) ||
			(contentType.includes('text/html') && isCloudflareBody(bodyText)) ||
			isCloudflareBody(bodyText)
		) {
			console.error(
				`Auth probe blocked by edge protection at ${API_URL}: ${response.status} ${contentType}`,
			);
			process.exit(20);
		}

		if (!response.ok) {
			console.error(`Auth probe reached API but login failed: ${response.status} ${bodyText}`);
			process.exit(1);
		}

		const body = JSON.parse(bodyText);
		if (!body.access_token || !body.csrf_token || !body.user) {
			console.error('Auth probe received JSON but required auth fields were missing.');
			process.exit(1);
		}

		console.debug(`Auth probe succeeded against ${API_URL}.`);
	} catch (error) {
		if (!isNetworkError(error)) {
			console.error(`Auth probe failed with a non-network error at ${API_URL}: ${error}`);
			process.exit(1);
		}
		console.error(`Auth probe could not reach ${API_URL}: ${error}`);
		process.exit(20);
	}
}

main();
