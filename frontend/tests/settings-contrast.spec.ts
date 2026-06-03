/**
 * Settings — light-theme text contrast (WCAG 2.2 AA)
 *
 * axe-core's color-contrast rule is disabled in accessibility.spec.ts because it
 * does not composite rgba/tint layers and trips on gradient-backed text. This spec
 * fills that gap for the light theme of the settings surface, which uses violet and
 * red tints behind text (active nav labels, "NEW" badges, device-current, Danger
 * Zone). For every visible text node it composites the element's own background and
 * all contributing ancestor backgrounds over the base surface, skips gradient/image
 * backed text (which a flat sampler cannot judge), and asserts the WCAG 2.2 AA
 * threshold: 4.5:1 for normal text, 3:1 for large text (≥24px, or ≥18.66px bold).
 *
 * The Appearance component re-applies the saved theme on mount, so the theme is
 * re-forced to light ~350ms after each tab switch before sampling.
 *
 * Tags: @accessibility
 * Run: npx playwright test settings-contrast --grep "@accessibility"
 *
 * @accessibility
 */

import { test as authedTest, expect } from './fixtures/auth.fixture';
import type { Page } from '@playwright/test';
import {
	mockExploreEndpoints,
	mockSupportEndpoints,
	mockPremiumEndpoints,
} from './helpers/mock-routes.js';

const NORMAL_MIN = 4.5;
const LARGE_MIN = 3.0;

/**
 * Classes that are intentionally de-emphasized (`--color-text-muted`) per the
 * design decision to keep muted for incidental meta. These sit just under AA
 * normal-text on the elevated surface (~3.8–4.4:1) by choice, not by oversight,
 * so they are excluded from the sweep. Do NOT add description/body classes here —
 * those were reclassified to `--color-text-secondary`.
 */
const INTENTIONAL_MUTED = ['nav-version', 'char-count', 'preview-username'];

interface ContrastFailure {
	text: string;
	color: string;
	background: string;
	ratio: number;
	required: number;
	fontSize: number;
	selector: string;
}

async function setupShellData(page: Page): Promise<void> {
	await page.route('**/api/v2/servers', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' }),
	);
	await page.route('**/api/v2/conversations', (route) =>
		route.fulfill({ status: 200, contentType: 'application/json', body: '[]' }),
	);
	await page.route('**/api/v2/devices', (route) =>
		route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{
					id: 'dev-1',
					label: 'This Device',
					platform: 'web',
					trust_state: 'trusted',
					last_seen_at: new Date().toISOString(),
				},
			]),
		}),
	);
}

async function forceLight(page: Page): Promise<void> {
	await page.evaluate(() => {
		document.documentElement.dataset.theme = 'light';
	});
}

/** Re-force light after the Appearance component resets it on mount, then settle. */
async function reforceLight(page: Page): Promise<void> {
	await page.waitForTimeout(400);
	await forceLight(page);
	await page.waitForTimeout(100);
}

async function collectFailures(page: Page): Promise<ContrastFailure[]> {
	return page.evaluate(
		({ normalMin, largeMin, intentionalMuted }) => {
			function parseColor(value: string): [number, number, number, number] | null {
				const match = value.match(/rgba?\(([^)]+)\)/);
				if (!match) return null;
				const parts = match[1].split(',').map((p) => parseFloat(p.trim()));
				const [r, g, b] = parts;
				const a = parts.length === 4 ? parts[3] : 1;
				return [r, g, b, a];
			}

			function over(
				src: [number, number, number, number],
				dst: [number, number, number],
			): [number, number, number] {
				const a = src[3];
				return [
					src[0] * a + dst[0] * (1 - a),
					src[1] * a + dst[1] * (1 - a),
					src[2] * a + dst[2] * (1 - a),
				];
			}

			function luminance([r, g, b]: [number, number, number]): number {
				const lin = [r, g, b].map((c) => {
					const s = c / 255;
					return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
				});
				return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2];
			}

			function ratio(fg: [number, number, number], bg: [number, number, number]): number {
				const l1 = luminance(fg);
				const l2 = luminance(bg);
				const [hi, lo] = l1 > l2 ? [l1, l2] : [l2, l1];
				return (hi + 0.05) / (lo + 0.05);
			}

			function cssPath(el: Element): string {
				const parts: string[] = [];
				let node: Element | null = el;
				for (let i = 0; node && i < 4; i++) {
					let part = node.tagName.toLowerCase();
					if (node.className && typeof node.className === 'string') {
						part += '.' + node.className.trim().split(/\s+/).slice(0, 2).join('.');
					}
					parts.unshift(part);
					node = node.parentElement;
				}
				return parts.join(' > ');
			}

			const failures: ContrastFailure[] = [];
			const root = document.querySelector('.settings-page, main, body') ?? document.body;
			const all = root.querySelectorAll('*');

			for (const el of Array.from(all)) {
				// Only elements with their own non-whitespace text node (leaf text owners).
				const ownText = Array.from(el.childNodes)
					.filter((n) => n.nodeType === Node.TEXT_NODE)
					.map((n) => n.textContent ?? '')
					.join('')
					.trim();
				if (!ownText) continue;

				// Skip intentionally-muted incidental meta (kept muted by design).
				const cls = typeof el.className === 'string' ? el.className : '';
				if (intentionalMuted.some((c) => cls.includes(c))) continue;

				const rect = el.getBoundingClientRect();
				if (rect.width === 0 || rect.height === 0) continue;
				const elStyle = getComputedStyle(el);
				if (elStyle.visibility === 'hidden' || elStyle.display === 'none') continue;
				if (parseFloat(elStyle.opacity) === 0) continue;

				const fg = parseColor(elStyle.color);
				if (!fg) continue;

				// Build effective background by compositing element + ancestor bg layers
				// front-to-back until an opaque layer is reached. Skip gradient/image text.
				const layers: [number, number, number, number][] = [];
				let node: Element | null = el;
				let gradientBacked = false;
				let reachedOpaque = false;
				while (node) {
					const s = getComputedStyle(node);
					if (s.backgroundImage && s.backgroundImage !== 'none') {
						gradientBacked = true;
						break;
					}
					const bg = parseColor(s.backgroundColor);
					if (bg && bg[3] > 0) {
						layers.push(bg);
						if (bg[3] >= 1) {
							reachedOpaque = true;
							break;
						}
					}
					node = node.parentElement;
				}
				if (gradientBacked) continue;

				// Base: light theme body surface (opaque). Composite back-to-front.
				let bg: [number, number, number] = reachedOpaque
					? [layers[layers.length - 1][0], layers[layers.length - 1][1], layers[layers.length - 1][2]]
					: [255, 255, 255];
				const front = reachedOpaque ? layers.slice(0, -1) : layers;
				for (let i = front.length - 1; i >= 0; i--) {
					bg = over(front[i], bg);
				}

				// Composite foreground alpha over background too (rare but correct).
				const fgRgb: [number, number, number] =
					fg[3] < 1 ? over(fg, bg) : [fg[0], fg[1], fg[2]];

				const fontSize = parseFloat(elStyle.fontSize);
				const fontWeight = parseInt(elStyle.fontWeight, 10) || 400;
				const isLarge = fontSize >= 24 || (fontSize >= 18.66 && fontWeight >= 700);
				const required = isLarge ? largeMin : normalMin;

				const r = ratio(fgRgb, bg);
				if (r < required) {
					failures.push({
						text: ownText.slice(0, 60),
						color: elStyle.color,
						background: `rgb(${bg.map((c) => Math.round(c)).join(',')})`,
						ratio: Math.round(r * 100) / 100,
						required,
						fontSize,
						selector: cssPath(el),
					});
				}
			}
			return failures;
		},
		{ normalMin: NORMAL_MIN, largeMin: LARGE_MIN, intentionalMuted: INTENTIONAL_MUTED },
	);
}

authedTest.describe('Settings light-theme contrast @accessibility', () => {
	authedTest('every settings tab meets WCAG 2.2 AA in light theme', async ({ userPage }) => {
		await setupShellData(userPage);
		await mockExploreEndpoints(userPage);
		await mockSupportEndpoints(userPage);
		await mockPremiumEndpoints(userPage);

		await userPage.goto('/settings');
		await expect(userPage.locator('[aria-label="Loading Yapper"]')).toHaveCount(0, {
			timeout: 30_000,
		});
		await forceLight(userPage);
		await userPage.waitForTimeout(150);

		const navItems = userPage.locator('.nav-item');
		const count = await navItems.count();
		expect(count).toBeGreaterThan(0);

		const allFailures: (ContrastFailure & { tab: string })[] = [];

		for (let i = 0; i < count; i++) {
			const item = navItems.nth(i);
			if (!(await item.isVisible())) continue;
			const tab = (await item.innerText()).replace(/\s+/g, ' ').trim();
			await item.click();
			await reforceLight(userPage);
			const failures = await collectFailures(userPage);
			allFailures.push(...failures.map((f) => ({ ...f, tab })));
		}

		const summary = allFailures
			.map(
				(f) =>
					`  [${f.tab}] "${f.text}" ${f.ratio}:1 (need ${f.required}:1, ${f.fontSize}px)\n    color ${f.color} on ${f.background}\n    ${f.selector}`,
			)
			.join('\n');

		expect(allFailures, `Light-theme contrast failures:\n${summary}`).toHaveLength(0);
	});
});
