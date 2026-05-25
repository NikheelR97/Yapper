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

  it("redacts sensitive values even when their keys are generic", () => {
    const sanitized = sanitizeSentryEvent({
      extra: {
        detail:
          "authorization Bearer eyJhbGciOiJSUzI1NiJ9.payload.signature",
        hash: "$argon2id$v=19$m=19456,t=2,p=1$abc$def",
        bundle: "identity_dh_key=abc signed_prekey=def",
      },
    });
    const json = JSON.stringify(sanitized);

    expect(json).not.toContain("Bearer");
    expect(json).not.toContain("eyJhbGciOiJSUzI1NiJ9");
    expect(json).not.toContain("$argon2id");
    expect(json).not.toContain("identity_dh_key");
    expect(json).not.toContain("signed_prekey");
    expect(json).toContain("[redacted]");
  });
});
