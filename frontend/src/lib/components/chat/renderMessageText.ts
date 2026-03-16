export type RenderedMessageToken =
  | { type: "text"; value: string }
  | { type: "emoji"; name: string; url: string };

const SHORTCODE_RE = /:[a-z0-9_]{2,32}:/g;

function isSafeEmojiUrl(value: string | undefined): value is string {
  if (!value) return false;
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:";
  } catch {
    return false;
  }
}

export function renderMessageTokens(
  text: string,
  emojiMap: ReadonlyMap<string, string>,
): RenderedMessageToken[] {
  const tokens: RenderedMessageToken[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(SHORTCODE_RE)) {
    const matched = match[0];
    const index = match.index ?? -1;
    if (index < 0) continue;

    if (index > lastIndex) {
      tokens.push({ type: "text", value: text.slice(lastIndex, index) });
    }

    const name = matched.slice(1, -1);
    const url = emojiMap.get(name);
    if (isSafeEmojiUrl(url)) {
      tokens.push({ type: "emoji", name, url });
    } else {
      tokens.push({ type: "text", value: matched });
    }

    lastIndex = index + matched.length;
  }

  if (lastIndex < text.length) {
    tokens.push({ type: "text", value: text.slice(lastIndex) });
  }

  return tokens.length ? tokens : [{ type: "text", value: text }];
}
