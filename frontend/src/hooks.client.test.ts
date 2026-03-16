import { describe, expect, it, vi } from "vitest";

vi.mock("@sentry/sveltekit", () => ({
  init: vi.fn(),
  handleErrorWithSentry: vi.fn(() => undefined),
}));

import { sanitizeSentryEvent } from "./hooks.client.js";

describe("sanitizeSentryEvent", () => {
  it("redacts common sensitive fields recursively", () => {
    const sanitized = sanitizeSentryEvent({
      request: {
        headers: {
          authorization: "Bearer secret-token",
        },
        data: {
          email: "alice@example.com",
          message: "super private body",
          nested: {
            password: "hunter2",
          },
        },
      },
    });

    expect(JSON.stringify(sanitized)).not.toContain("secret-token");
    expect(JSON.stringify(sanitized)).not.toContain("alice@example.com");
    expect(JSON.stringify(sanitized)).not.toContain("super private body");
    expect(JSON.stringify(sanitized)).not.toContain("hunter2");
    expect(JSON.stringify(sanitized)).toContain("[redacted]");
  });
});
