# Codex Security Report

Reviewed on: 2026-03-16

Repository: `d:\Development\Claude\yapper`

Primary stack observed during review:
- Backend: Rust + Axum + SQLx
- Frontend: SvelteKit + TypeScript
- Clients: Web, Tauri desktop, Capacitor mobile
- Deployment model: containerized runtime with multi-stage Docker build

Overall risk rating: **High**

## Executive Summary

Yapper is handling PII and sensitive account metadata in a way that is already beyond a toy-app threat model. The codebase has several strong foundations, including parameterized SQLx queries, a CSRF double-submit control, a non-root runtime container, and hashed bot tokens at rest. Those controls are real and worth keeping.

The most important problem is that the trusted-device model is not enforced consistently on the server. New devices can be created in `pending_trust`, receive valid JWTs, and still reach multiple legacy `/api/v1` endpoints that use `AuthUser` instead of `AuthDevice`. That turns device approval into a partial UI workflow rather than a hard authorization boundary.

The second major problem is that E2EE and local keystore handling fail open in the client. When secure storage setup fails, the app continues, and the IndexedDB encryption key is stored in `localStorage`. For a PII-bearing messaging product, this is not an enterprise-grade posture.

The third major problem is a business-logic flaw in backup restore: the current flow appears to trust the requesting device and then revoke the source trusted device. If that is not a deliberate, explicitly confirmed "replace device" flow, it creates avoidable account disruption and recovery risk.

In short: this prototype has the shape of a modern secure system, but several AI-generated convenience patterns are undermining the intended security model. Those issues should be treated as release blockers before broad exposure.

## Scope and Methodology

### Reviewed inputs

Documentation reviewed:
- `README.md`
- `wiki-repo/Architecture.md`
- `wiki-repo/Security.md`
- `wiki-repo/API-Reference.md`
- `wiki-repo/Backend-Development.md`
- `wiki-repo/Frontend-Development.md`
- `wiki-repo/Database.md`
- `wiki-repo/Deployment.md`
- `dev docs/HANDOVER.md`
- `dev docs/SPRINT_PLAN.md`

Code areas reviewed in detail:
- Authentication, sessions, and middleware
- Device trust and key backup flows
- Message rendering and client keystore bootstrap
- Support ticket handling and external integrations
- Error handling and telemetry
- Container runtime configuration

### Validation performed

Baseline checks executed during review:

```powershell
cd d:\Development\Claude\yapper\frontend
npm test -- --run
```

Result:
- Existing frontend tests passed: 10 files, 33 tests passed

```powershell
cd d:\Development\Claude\yapper
cargo test --manifest-path backend\Cargo.toml --quiet --target-dir backend\target-codex
```

Result:
- Existing backend tests passed when run with an isolated target directory
- The default target directory had a locked `backend\target\debug\yapper-server.exe`, so the isolated target dir was used to avoid a false negative

### Assessment limits

- This was a source review with targeted static validation, not a live external penetration test against a deployed environment
- Kubernetes manifests, cloud IAM, WAF policy, secret manager policy, and vendor configurations were not fully available in-repo
- Findings below are therefore confirmed code-level issues and architecture risks, not a complete infrastructure attestation

## Severity Matrix

| ID | Severity | Category | Summary |
| --- | --- | --- | --- |
| YAP-SEC-001 | High | Broken Access Control / CWE-285 / CWE-862 | `pending_trust` devices can still access multiple privileged legacy `/api/v1` routes |
| YAP-SEC-002 | High | Insecure Design / Business Logic Error / CWE-841 | Backup restore appears to revoke the trusted source device after approving the new device |
| YAP-SEC-003 | High | Protection Mechanism Failure / Insecure Storage / CWE-693 / CWE-922 | Client crypto bootstrap fails open and local encryption material is stored in `localStorage` |
| YAP-SEC-004 | Medium | XSS / CWE-79 | Message rendering uses `{@html}` and injects emoji URLs into raw HTML |
| YAP-SEC-005 | Medium | PII Exposure / Logging and Third-Party Egress / CWE-359 / CWE-532 | HubSpot forwarding and telemetry capture can move user identifiers and sensitive content outside the minimum necessary scope |

## Detailed Findings

### YAP-SEC-001 - Trusted-device enforcement is bypassed on legacy v1 routes

- Severity: High
- Category: OWASP A01 Broken Access Control, CWE-285 Improper Authorization, CWE-862 Missing Authorization
- Affected surface: account data, messaging metadata/history, key backup material, support flows, and other legacy authenticated endpoints under `/api/v1`

#### Evidence

- `backend/src/auth/middleware.rs:24-57`
  `AuthUser` accepts any valid bearer token and does not inspect device trust state.
- `backend/src/auth/middleware.rs:62-95`
  `AuthDevice` exists separately and is capable of enforcing trust state.
- `backend/src/auth/middleware.rs:71-77`
  `require_trusted()` explicitly blocks non-trusted devices.
- `backend/src/devices/mod.rs:198-214`
  Subsequent devices default to `pending_trust`, not `trusted`.
- `backend/src/auth/v2.rs:202-204`
  Login still mints a session for the newly registered or reused device.
- `backend/src/auth/v2.rs:366-399`
  `issue_device_session()` issues both access and refresh tokens before trust is enforced at route level.
- Representative legacy routes still using `AuthUser`:
  - `backend/src/messages/mod.rs:51-56`
  - `backend/src/messages/mod.rs:157-161`
  - `backend/src/messages/mod.rs:278-284`
  - `backend/src/keys/mod.rs:460-497`
  - `backend/src/users/mod.rs:66-100`
  - `backend/src/users/mod.rs:1248-1424`
  - `backend/src/support/mod.rs:121-219`

#### Exploit scenario

1. An attacker obtains valid credentials or an approved OAuth login for a victim account.
2. The attacker signs in from a new device, which is correctly marked `pending_trust`.
3. The server still issues a valid JWT for that device.
4. The attacker calls legacy `/api/v1` endpoints that authorize via `AuthUser` instead of `AuthDevice`.
5. The device approval model is bypassed for large parts of the application.

#### Business impact

- Exposure of PII and account metadata before device approval
- Unauthorized access to conversation history and account export surfaces
- Undermining of the multi-device trust model that the product appears to rely on for E2EE safety
- High likelihood of audit and compliance failure because the documented trust boundary is not the actual enforced boundary

#### Why this looks AI-generated

- Two adjacent auth abstractions exist, but only one actually enforces the security property
- Security depends on developers remembering which extractor to use per route
- The protected workflow is present in the product, but enforcement is inconsistent and easy to miss in future endpoints

#### Recommended remediation

Immediate:
- Treat device trust as a server-enforced authorization boundary, not a UX hint
- Change every sensitive authenticated route from `AuthUser` to `AuthDevice` plus `auth.require_trusted()?`
- Explicitly enumerate the very small set of routes that may be reachable by `pending_trust` devices, such as device bootstrap, approval polling, and logout

Short term:
- Freeze or deprecate privileged `/api/v1` routes if `/api/v2` already has the trusted-device-aware equivalent
- Add a route policy matrix in docs and tests that marks each endpoint as one of:
  - public
  - authenticated-any-device
  - authenticated-trusted-device-only

Performance-aware improvement:
- If the team is concerned about extra device lookups on hot paths, use short-lived access tokens plus a server-side session or trust-version check, rather than dropping trust enforcement
- A cached trust/session version in Redis or a short TTL claim refreshed from the DB is safer than routing privileged traffic through `AuthUser`

#### Verification steps

- Create a second device for an existing account and confirm the server marks it `pending_trust`
- Attempt calls to `/api/v1/account/data-export`, `/api/v1/conversations`, `/api/v1/keys/backup`, and `/api/v1/users/me`
- Expected secure result after remediation: `403 Forbidden` or `401 Unauthorized` for all privileged routes until device approval completes

### YAP-SEC-002 - Backup restore flow appears to revoke the trusted source device

- Severity: High
- Category: OWASP A04 Insecure Design, CWE-841 Improper Enforcement of Behavioral Workflow
- Affected surface: device recovery, session continuity, key backup ownership, and device trust lifecycle

#### Evidence

- `backend/src/keys/mod.rs:893-916`
  The restore flow requires a different `source_device_id` and requires that source device to already be trusted.
- `backend/src/keys/mod.rs:936-950`
  The current requesting device is promoted to `trusted`, approved by the source device.
- `backend/src/keys/mod.rs:965-975`
  The source device is then marked `trust_state = 'revoked'`.
- `backend/src/keys/mod.rs:978-994`
  Sessions and backups for the source device are revoked.
- `backend/src/keys/mod.rs:1021-1024`
  The response returns `"replaced_device_id": req.source_device_id`.

#### Exploit scenario

1. A user attempts to restore a backup from a trusted device to a new device.
2. The code promotes the new device.
3. The old trusted source device is immediately revoked and all sessions and backups for that source are invalidated.
4. If this was not an explicitly confirmed device replacement operation, the system destroys the original trusted path as a side effect of restore.

#### Business impact

- Unexpected device lockout
- Recovery and support burden
- Possible loss of continuity if the source device was still legitimately in use
- Risk of malicious account takeover becoming more durable by revoking the legitimate source device during recovery

#### Why this looks AI-generated

- A "restore" workflow has been conflated with a "replace and revoke" workflow
- The code is logically consistent but semantically unsafe
- The response shape suggests replacement semantics, but that is not surfaced as a distinct, audited operation

#### Recommended remediation

Immediate:
- Split restore into two explicit modes:
  - `restore_only`: import keys to the current device and keep source trusted
  - `replace_device`: import keys and revoke the source device only after explicit confirmation and audit logging
- Make `restore_only` the default behavior

Short term:
- Require a server-side audit event for any destructive device replacement
- Record actor device, target device, reason, and user-visible confirmation marker
- Prevent backup revocation unless the destructive replacement mode is selected

#### Verification steps

- Restore a backup from a trusted source to a pending device
- Expected secure result after remediation:
  - current device becomes trusted
  - source device remains trusted in normal restore mode
  - source device is only revoked in explicit replacement mode
  - session and backup revocations occur only in replacement mode

### YAP-SEC-003 - Client cryptography fails open and stores local protection material unsafely

- Severity: High
- Category: OWASP A02 Cryptographic Failures, CWE-693 Protection Mechanism Failure, CWE-922 Insecure Storage of Sensitive Information
- Affected surface: client E2EE bootstrap, local IndexedDB protection, browser storage of crypto material, installation identity persistence

#### Evidence

- `frontend/src/routes/(app)/+layout.svelte:811-816`
  If keystore configuration fails, the app logs the failure and continues: `"continuing without E2EE"`.
- `frontend/src/lib/signal/idbCrypto.ts:74-96`
  The raw AES key for IndexedDB encryption is exported and stored in `localStorage`.
- `frontend/src/lib/signal/idbCrypto.ts:100-112`
  Initialization failures clear the crypto state and proceed unencrypted.
- `frontend/src/lib/device/bootstrap.ts:61`
  A stable `installation_id` is stored in `localStorage`.

#### Exploit scenario

1. The browser cannot initialize WebCrypto or IndexedDB encryption, or the key material is corrupted.
2. The application continues rather than blocking secure functionality.
3. Sensitive local E2EE state is left unencrypted or recoverable from `localStorage`.
4. Any XSS, extension compromise, kiosk browser compromise, or local malware can recover the at-rest key and bootstrap identity.

#### Business impact

- Silent downgrade from intended secure mode to insecure mode
- Weak local protection of messaging and key material
- Higher blast radius for any client-side compromise
- Hard-to-detect loss of confidentiality for a product that advertises secure messaging behavior

#### Why this looks AI-generated

- UX continuity is prioritized over security invariants
- A protective control exists, but failure handling intentionally disables it instead of stopping the workflow
- The local encryption key is stored in a way that negates much of the value of the encryption layer

#### Recommended remediation

Immediate:
- Fail closed when `configureActiveSignalStore()` or `initIdbEncryption()` fails
- Do not start sync, key upload, or message workflows until the secure store is initialized
- Surface a blocking recovery flow rather than a non-fatal toast

Short term:
- On web, do not export and persist the raw local encryption key in `localStorage`
- Prefer:
  - a non-exportable WebCrypto key when feasible
  - platform keystores for Tauri and mobile clients
  - a user-unlocked vault or passphrase-derived wrapping key for browser-only recovery flows
- Treat `installation_id` as a device identity artifact and store it in the same protected storage domain where possible

Architecture note:
- If a secure browser-only keystore cannot meet the threat model, narrow the claim set for the web client and document that only desktop/mobile clients meet the full E2EE assurance target

#### Verification steps

- Force `initIdbEncryption()` to throw and confirm protected views do not initialize
- Inspect browser storage and verify raw encryption material is not recoverable from `localStorage`
- Confirm desktop/mobile variants use OS-backed secure storage

### YAP-SEC-004 - Message rendering uses a raw HTML sink

- Severity: Medium
- Category: OWASP A03 Injection, CWE-79 Cross-site Scripting
- Affected surface: chat message rendering in direct messages and channels

#### Evidence

- `frontend/src/lib/components/chat/MessageList.svelte:28-42`
  `renderText()` escapes message text but then injects emoji URLs into a raw `<img>` string.
- `frontend/src/lib/components/chat/MessageList.svelte:129`
  The rendered string is inserted with `{@html renderText(msg.text)}`.

#### Exploit scenario

The current implementation escapes the text content itself, which is good. The residual risk is the raw HTML sink combined with dynamic `src` attribute construction. If `imageUrl` ever becomes attacker-controlled, insufficiently normalized, or sourced from an untrusted tenant path, this becomes a practical XSS or content injection path.

#### Business impact

- Chat-rendering XSS can expose tokens, local crypto material, and user data
- Because the client already stores sensitive state locally, a UI XSS issue would have a large blast radius
- This is exactly the kind of "small convenience helper" issue that expands later as features grow

#### Recommended remediation

Immediate:
- Remove `{@html}` from message rendering for user-controlled content paths
- Replace it with tokenization plus structured Svelte rendering

Short term:
- Validate emoji/media URLs against a strict allowlist of schemes and domains
- Reject `javascript:`, `data:`, and other non-approved schemes
- Add a restrictive CSP for web surfaces to reduce exploitability

#### Verification steps

- Test with malicious emoji URLs, malformed attributes, and encoded payloads
- Confirm no script execution and no DOM mutation outside intended `<img>` nodes

### YAP-SEC-005 - PII minimization is insufficient in HubSpot forwarding and telemetry capture

- Severity: Medium
- Category: CWE-359 Exposure of Private Personal Information, CWE-532 Information Exposure Through Log Files
- Affected surface: support ticket export to HubSpot, backend exception capture, frontend Sentry replay capture

#### Evidence

- `backend/src/support/mod.rs:143-175`
  Support ticket creation fetches user email and username, stores the full description locally, and forwards content to HubSpot.
- `backend/src/support/mod.rs:278-299`
  HubSpot ticket content includes:
  `Submitted by: @{username} ({user_email})`
- `backend/src/error.rs:46-57`
  Internal and database errors are logged and captured to Sentry.
- `frontend/src/hooks.client.ts:10-21`
  Sentry replays are enabled on error with `replaysOnErrorSampleRate: 1.0`.

#### Exploit scenario

1. A user submits a support ticket containing sensitive free-text content.
2. That content, plus username and email, is forwarded to HubSpot.
3. An application error on a sensitive page causes a replay or stack context to be captured to Sentry.
4. The effective data-sharing boundary becomes broader than the minimum necessary operational boundary.

#### Business impact

- Greater vendor risk surface
- Increased scope for GDPR processor controls and retention review
- Potential accidental collection of payment data, health data, or other regulated content if users type it into support forms
- Harder evidence posture for SOC 2 if redaction and allowlisting are not formalized

#### Recommended remediation

Immediate:
- Apply field-level allowlisting for third-party forwarding
- Do not forward raw free-text support descriptions to third parties by default if the product does not need them there
- If free text must be forwarded, redact or tokenize direct identifiers first

Short term:
- Add `beforeSend` and replay scrubbing hooks for Sentry
- Disable replay capture on sensitive pages until explicit DOM masking and network/body scrubbing are verified
- Classify support ticket data and define retention, deletion, and processor responsibilities for HubSpot

Operational hardening:
- Add a support form warning instructing users not to submit passwords, payment data, or health data
- If HIPAA or PCI scope is possible, do not permit unfiltered support free text to enter non-approved vendors

#### Verification steps

- Submit synthetic tickets containing seeded test PII and confirm the outbound HubSpot payload is redacted
- Generate backend and frontend errors with synthetic markers and confirm Sentry receives scrubbed payloads only

## Strengths and Non-Findings

The review did identify working controls that should be preserved:

- No confirmed SQL injection path was found in the inspected areas because SQLx parameter binding is being used consistently rather than string concatenation
  - Examples: `backend/src/support/mod.rs:130-162`, `backend/src/users/mod.rs:107-174`
- CSRF protection exists and is applied via a double-submit cookie pattern
  - `backend/src/main.rs:258-267`
  - `backend/src/csrf.rs:17-50`
- The runtime container already avoids running as root
  - `backend/Dockerfile:21-39`
- Bot tokens appear hashed before storage rather than persisted in plaintext
  - `backend/src/bots/mod.rs:11`
  - `backend/src/bots/mod.rs:225-229`

Additional observation:
- No confirmed hard-coded private production API secrets were identified in the sampled source review
- Continue automated secret scanning anyway because this repository contains multiple app surfaces and CI artifacts

## AI Slop Patterns Observed

These are recurring patterns that often appear in AI-assisted code generation and were visible here:

1. Security properties exist in architecture docs but are not enforced uniformly in code.
2. Security-sensitive failures are converted into UX-friendly warnings instead of hard stops.
3. Workflow code blends separate concepts such as restore, replace, revoke, and approve.
4. Framework safety is bypassed for convenience, such as raw HTML rendering shortcuts.
5. Third-party integrations are fed rich context by default instead of a minimum necessary payload.

Recommended engineering guardrails:

- Add a "security invariant" checklist to PR reviews for auth, crypto, storage, logging, and third-party integrations
- Require explicit threat-model notes when introducing:
  - new auth extractors
  - raw HTML sinks
  - browser storage of identifiers or secrets
  - best-effort data forwarding to vendors
- Treat AI-generated code as untrusted until it passes a secure coding review, especially around auth, crypto, and serialization boundaries

## Architecture and Deployment Hardening

### Code and API design

- Collapse privileged functionality onto a single trusted-device-aware authorization model
- Mark every endpoint with an explicit auth policy and make the unsafe/default policy impossible to choose accidentally
- Separate "any authenticated device" routes from "trusted device required" routes at the router level, not just per handler

### Cryptography and local state

- Define a per-platform keystore strategy:
  - Web: strongest achievable browser keystore with documented residual risk
  - Tauri: OS-backed secret storage
  - Mobile: platform keystore or secure enclave equivalent
- Keep E2EE invariants fail-closed
- Do not store recoverable wrapping keys beside the encrypted data they protect

### Data governance

- Create a data classification table for user profile data, messaging metadata, encrypted content, support tickets, telemetry, and audit logs
- Enforce "minimum necessary" data flow to vendors
- Add scrubbers for logs, traces, replays, support exports, and analytics

### Container and runtime posture

Current state:
- Multi-stage build
- Non-root runtime user

Recommended next steps:
- Pin base images by digest instead of floating tags
- Run containers with a read-only root filesystem where feasible
- Drop all unnecessary Linux capabilities
- Enable seccomp/AppArmor defaults in deployment
- Mount secrets from a secret manager, not baked environment files
- Add network policy / egress restrictions for only required third-party endpoints

### Supply chain and CI

- Add automated secret scanning, dependency auditing, SBOM generation, and image scanning on every merge
- Fail CI on newly introduced critical vulnerabilities unless formally waived
- Keep a small approved-vendors list with owner, purpose, data classes sent, and retention period

## Security Test Scripts

The following snippets are intended as directly reusable starting points for the repo. They are written to match the current stack and the issues found during review.

### 1. Rust regression test for pending-trust authorization

Recommended location: `backend/tests/security_authz.rs`

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn pending_trust_device_cannot_access_privileged_v1_routes() {
    let harness = test_harness::spawn().await;

    let victim = harness.register_user("victim@example.com", "Alpha2468!").await;
    let trusted = harness.login_device(&victim, "trusted-device").await;
    let pending = harness.login_device(&victim, "pending-device").await;

    assert_eq!(pending.device.trust_state, "pending_trust");

    let privileged_paths = [
        "/api/v1/account/data-export",
        "/api/v1/conversations",
        "/api/v1/keys/backup",
        "/api/v1/users/me",
        "/api/v1/support/tickets",
    ];

    for path in privileged_paths {
        let response = harness
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .method("GET")
                    .header("authorization", format!("Bearer {}", pending.access_token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::FORBIDDEN
                || response.status() == StatusCode::UNAUTHORIZED,
            "path {path} unexpectedly returned {}",
            response.status()
        );
    }

    let trusted_response = harness
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/users/me")
                .method("GET")
                .header("authorization", format!("Bearer {}", trusted.access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(trusted_response.status(), StatusCode::OK);
}
```

### 2. Rust regression test for backup restore semantics

Recommended location: `backend/src/keys/mod.rs` test module or `backend/tests/security_restore.rs`

```rust
#[tokio::test]
async fn restore_only_does_not_revoke_the_source_device() {
    let harness = test_harness::spawn().await;
    let user = harness.register_user("restore@example.com", "Alpha2468!").await;

    let source = harness.login_device(&user, "desktop").await;
    let target = harness.login_device(&user, "phone").await;
    assert_eq!(target.device.trust_state, "pending_trust");

    harness.upload_backup(&source, "encrypted-blob").await;
    harness.restore_backup(&target, source.device.id).await;

    let source_device = harness.fetch_device(source.device.id).await;
    let target_device = harness.fetch_device(target.device.id).await;

    assert_eq!(source_device.trust_state, "trusted");
    assert!(source_device.revoked_at.is_none());
    assert_eq!(target_device.trust_state, "trusted");
}

#[tokio::test]
async fn replace_device_mode_requires_explicit_confirmation_and_revokes_source() {
    let harness = test_harness::spawn().await;
    let user = harness.register_user("replace@example.com", "Alpha2468!").await;

    let source = harness.login_device(&user, "desktop").await;
    let target = harness.login_device(&user, "phone").await;

    harness.upload_backup(&source, "encrypted-blob").await;
    harness.restore_backup_with_replace(&target, source.device.id, true).await;

    let source_device = harness.fetch_device(source.device.id).await;
    let target_device = harness.fetch_device(target.device.id).await;

    assert_eq!(target_device.trust_state, "trusted");
    assert_eq!(source_device.trust_state, "revoked");
    assert!(source_device.revoked_at.is_some());
}
```

### 3. Vitest for message rendering safety

Recommended refactor first:
- Extract `renderText()` from `MessageList.svelte` into a pure helper such as `frontend/src/lib/components/chat/renderMessageText.ts`
- Return structured tokens, not raw HTML strings

Recommended test location: `frontend/src/lib/components/chat/renderMessageText.test.ts`

```ts
import { describe, expect, it } from "vitest";
import { renderMessageTokens } from "./renderMessageText";

describe("renderMessageTokens", () => {
  it("does not create executable HTML from attacker-controlled text", () => {
    const tokens = renderMessageTokens(
      `hello <img src=x onerror=alert(1)> :party:`,
      new Map([["party", "https://cdn.yapperhq.example/emojis/party.png"]]),
    );

    expect(tokens).toEqual([
      { type: "text", value: "hello <img src=x onerror=alert(1)> " },
      {
        type: "emoji",
        name: "party",
        url: "https://cdn.yapperhq.example/emojis/party.png",
      },
    ]);
  });

  it("rejects non-allowlisted emoji URLs", () => {
    expect(() =>
      renderMessageTokens(
        ":party:",
        new Map([["party", "javascript:alert(1)"]]),
      ),
    ).toThrow(/invalid emoji url/i);
  });
});
```

### 4. Vitest for fail-closed keystore bootstrap

Recommended location: `frontend/src/lib/signal/keystore-fail-closed.test.ts`

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./idbCrypto.js", () => ({
  initIdbEncryption: vi.fn(async () => {
    throw new Error("crypto unavailable");
  }),
}));

import { configureSignalStore } from "./keystore.js";

describe("signal bootstrap", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("fails closed when IndexedDB encryption cannot initialize", async () => {
    await expect(
      configureSignalStore({ userId: "user-1", deviceId: "device-1" }),
    ).rejects.toThrow(/crypto unavailable/i);
  });
});
```

### 5. Redaction tests for HubSpot and Sentry payloads

Recommended direction:
- Introduce pure payload-builder helpers before testing
- Keep all scrubbing logic out of UI components and handler bodies

Recommended test cases:

```rust
#[test]
fn hubspot_payload_redacts_direct_identifiers_by_default() {
    let payload = build_hubspot_ticket_payload(
        "idea",
        "Need help",
        "My SSN is 123-45-6789",
        "high",
        "alice@example.com",
        "alice",
        true,
    );

    let content = payload["properties"]["content"].as_str().unwrap();
    assert!(!content.contains("alice@example.com"));
    assert!(!content.contains("123-45-6789"));
}
```

```ts
import { describe, expect, it } from "vitest";
import { sanitizeSentryEvent } from "./telemetry";

describe("sanitizeSentryEvent", () => {
  it("removes tokens, emails, and message bodies before submission", () => {
    const sanitized = sanitizeSentryEvent({
      request: {
        headers: {
          authorization: "Bearer secret-token",
        },
        data: {
          email: "alice@example.com",
          message: "super private body",
        },
      },
    });

    expect(JSON.stringify(sanitized)).not.toContain("secret-token");
    expect(JSON.stringify(sanitized)).not.toContain("alice@example.com");
    expect(JSON.stringify(sanitized)).not.toContain("super private body");
  });
});
```

### 6. CI security scanning commands

Add the following to CI as first-class security jobs:

```bash
# Secrets
gitleaks dir . --verbose

# Rust dependencies
cargo audit

# JavaScript dependencies
npm audit --audit-level=high

# Multi-ecosystem vulnerability scan
osv-scanner -r .

# SBOM generation
syft . -o cyclonedx-json > sbom.json

# Filesystem and image scanning
trivy fs --scanners vuln,secret,config .
trivy image yapper:local
```

Recommended policy:
- block merges on newly introduced critical findings
- require explicit waiver records for accepted risk

## Compliance Notes

### GDPR

Most directly applicable based on the observed product behavior:
- PII is clearly in scope
- Data export and deletion features already exist, which is positive
- Remaining gaps are around minimization, third-party forwarding, redaction, and formal vendor/process controls

### SOC 2

Most relevant control themes from this review:
- least privilege and authorization consistency
- logging and monitoring hygiene
- vendor and subprocessor management
- secure SDLC and automated scanning evidence

### PCI-DSS

Conditionally applicable:
- If support tickets or any product surface can receive payment card data, HubSpot forwarding and replay capture become unacceptable without strict filtering
- Add user-facing warnings and server-side rejection patterns for obvious PAN-like content if payment scope exists

### HIPAA

Conditionally applicable:
- If the platform may collect PHI, do not forward sensitive support data or telemetry to vendors without BAAs and documented permitted use
- Current best-effort forwarding and replay capture would need a much stricter data handling design

## Prioritized Remediation Roadmap

### P0 - Release blockers

- Enforce trusted-device authorization on every privileged route
- Stop the client from continuing when secure keystore bootstrap fails
- Remove implicit source-device revocation from normal backup restore
- Reduce outbound PII in HubSpot and Sentry immediately

### P1 - Next sprint

- Replace raw HTML message rendering with tokenized safe rendering
- Add the regression tests listed above
- Add CI security scanning and secret detection
- Publish a route-level authorization matrix and a data-flow inventory

### P2 - Hardening track

- Unify auth/versioning so privileged features live behind one consistent policy model
- Implement platform-appropriate secure keystore storage
- Pin and harden container images and deployment runtime policy
- Formalize vendor data governance and retention controls

## Final Assessment

The codebase is not in "public exploit ready" shape yet for a PII-bearing messaging product, even though it already contains several good security building blocks. The combination of trust-boundary bypass, fail-open client crypto, and destructive restore semantics means the current prototype should be treated as **high risk** until those issues are closed and covered by regression tests.

The good news is that the problems are remediable without a full rewrite. The shortest path to an enterprise-grade posture is to make security invariants impossible to bypass accidentally: enforce device trust on the server, fail closed on local crypto setup, treat restore and replacement as separate operations, minimize third-party data flow, and add regression tests that lock those guarantees in place.
