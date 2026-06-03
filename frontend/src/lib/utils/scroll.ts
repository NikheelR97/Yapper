/**
 * Shared scroll-anchoring helpers for chat-style message views (channel + DM).
 *
 * The behaviour these encode — "follow new messages only while the reader is
 * already at the bottom" — used to be copy-pasted into each route, which is how
 * the DM and channel views drifted apart. Keep the logic here so both views
 * stay in lockstep.
 */

/** Distance from the bottom (px) within which the reader still counts as "at bottom". */
export const NEAR_BOTTOM_PX = 120;

export function prefersReducedMotion(): boolean {
	return (
		typeof window !== "undefined" &&
		window.matchMedia("(prefers-reduced-motion: reduce)").matches
	);
}

/**
 * True when the scroll container is within `threshold` px of its bottom, or
 * when it isn't mounted yet (so first paint is treated as "at bottom").
 */
export function isNearBottom(
	el: HTMLElement | null | undefined,
	threshold: number = NEAR_BOTTOM_PX,
): boolean {
	if (!el) return true;
	return el.scrollHeight - el.scrollTop - el.clientHeight < threshold;
}
