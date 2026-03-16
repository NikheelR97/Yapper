import { describe, expect, it } from "vitest";

import { renderMessageTokens } from "./renderMessageText.js";

describe("renderMessageTokens", () => {
  it("preserves plain text as text tokens", () => {
    expect(renderMessageTokens("hello world", new Map())).toEqual([
      { type: "text", value: "hello world" },
    ]);
  });

  it("renders safe emoji URLs as structured emoji tokens", () => {
    expect(
      renderMessageTokens(
        "hi :party:",
        new Map([["party", "https://cdn.test/party.png"]]),
      ),
    ).toEqual([
      { type: "text", value: "hi " },
      { type: "emoji", name: "party", url: "https://cdn.test/party.png" },
    ]);
  });

  it("leaves unsafe emoji URLs as plain text", () => {
    expect(
      renderMessageTokens(
        "look :party:",
        new Map([["party", "javascript:alert(1)"]]),
      ),
    ).toEqual([
      { type: "text", value: "look " },
      { type: "text", value: ":party:" },
    ]);
  });
});
