# Yapper Security Assessment Report
**Classification:** CONFIDENTIAL - PRE-LAUNCH
**Assessor:** Principal Security Researcher / White Hat Pen Tester
**Assessment date:** 2026-03-24
**Codebase state:** S16 complete (rev 4, 2026-03-22) - S15 launch in progress
**Prior audit baseline:** 0 Critical, 0 High, 4 Medium, 5 Low, 4 Info (post-S14)

---

## Executive Summary
Yapper's current security posture is materially improved from the earlier S15 draft. The previously confirmed launch-blocking issues in backup restore, DM/bootstrap authorization, refresh-token rotation, media upload enforcement, canvas membership checks, key-bundle access control, OAuth unlink safety, and HTTP ciphertext-size handling are now remediated in the code reviewed on 2026-03-24.

I did not confirm an active break of X3DH, Double Ratchet state handling, sender-key confidentiality, or client-side media encryption in the current implementation. The server still preserves the core architectural constraint that it never needs plaintext message content to enforce the fixes shipped in this pass.

The remaining risk is primarily regression risk rather than a currently confirmed exploitable gap. GDPR portability and erasure are substantially better than before because export coverage is broader, linked operational records are deleted on account removal, a retention cleanup task now purges stale sessions, tokens, sync events, consumed OPKs, revoked device backups, and expired media bookkeeping, and deleted accounts now enter a documented 30-day hold before the cleanup worker either hard-deletes the user or explicitly retains only an anonymized shell when referential-integrity constraints still require it.

Launch readiness is now materially better. Based on the code reviewed in this workspace, I do not see an open Critical or High finding that should still block S15, and I do not see a remaining open Medium finding in the remediated assessment set. The main post-launch focus is preserving regression coverage and keeping the retention/docs policy aligned with future schema changes.

## Overall Risk Score
**3.1 / 10 (LOW)** using a highest-open-risk-plus-density approach aligned to CVSS v3.1 base scoring. The score is no longer Medium because the previously open GDPR lifecycle gap is now remediated with explicit retention metadata, a documented hold window, and conditional hard deletion for eligible deleted accounts. Residual risk is now dominated by normal regression potential rather than a confirmed active control failure.

## Finding Inventory
| ID | Title | Layer | Severity | CVSS | Compliance | Launch Blocker? |
|----|-------|-------|----------|------|------------|-----------------|
| N/A | No open confirmed findings remain in the reviewed code as of 2026-03-24 | N/A | N/A | N/A | N/A | No |

---

## Findings

No open confirmed findings remain after the remediations reviewed on 2026-03-24. The current state reflects:

- account deletion that purges linked operational rows, anonymizes the remaining `users` shell, and stamps explicit retention metadata
- a retention worker that hard-deletes eligible deleted users after the 30-day hold and records a referential-integrity basis when an anonymized shell must remain
- synchronized security/API documentation plus a CI docs-sync guard to keep the published control inventory aligned with implementation

Residual risk is limited to future regression or schema drift rather than a confirmed active vulnerability.

---

## Compliance Gap Summary
| Standard | Control | Current State | Gap | Remediation Owner | Target State |
|----------|---------|---------------|-----|-------------------|-------------|
| COPPA | Child contact requires verifiable parental approval | DM bootstrap and key-bundle access now share the same server-side access gate as relationship policy | No material open gap confirmed in reviewed code | Backend auth / parental controls | Keep regression coverage on DM and key-bootstrap policy paths |
| GDPR | Article 17 / Article 20 | Export is materially broader; account deletion stamps a 30-day hold plus retention basis; retention cleanup hard-deletes eligible deleted users and retains only anonymized shells when required by referential integrity | No material open gap confirmed in reviewed code | Backend data platform | Keep retention metadata and docs aligned with future schema changes |
| SOC2 | CC6 Logical access | Sensitive auth and canvas authorization gaps reviewed in this pass are remediated | Residual risk is regression rather than an active open control failure | Backend platform | Preserve centralized authorization helpers with regression tests |
| SOC2 | CC7 / CC8 Operations and change management | Sentry scrubbing exists; cargo/npm audits block; docs-sync CI guard now checks key security-doc invariants | No material open gap confirmed in reviewed docs/CI controls | Security engineering | Keep the docs-sync check current as routes and exemptions evolve |
| PCI-DSS | SAQ-A / webhook integrity | Stripe webhook signature verification exists in `premium/service.rs` | No material gap confirmed in reviewed code | Payments integration | Keep existing verification and document scope boundaries |

## Launch Readiness Assessment
### Pre-Launch Blockers (must fix before S15 completes)
1. No open Critical or High findings confirmed in the reviewed code on 2026-03-24.

### Post-Launch Priority Queue (fix within 30 days of launch)
1. No mandatory post-launch remediation item remains from this assessment pass.

### Backlog (fix within 90 days)
1. Periodically re-evaluate whether advisory OSV/Trivy scans should become blocking as release risk and audit pressure increase.

## Prioritized Remediation Roadmap
| Priority | Finding ID | Estimated Effort | Risk if Deferred |
|----------|------------|-----------------|-----------------|
| P4 | Advisory scan posture | 1 day | High/critical dependency or image findings may remain informational longer than desired as audit pressure increases |

## Appendix A - S16 Canvas Expansion Audit Checklist
| Area / Endpoint Group | Authorization Result | Validation Result | Notes |
|-----------------------|----------------------|------------------|-------|
| Music state / queue / history / settings / DJ management | Pass | Pass | Handlers consistently call `require_member`, `require_admin_or_dj`, or `require_server_admin`; queue capped at 50 (`MAX_MUSIC_QUEUE_SIZE`) |
| Poll creation | Pass | Pass | Channel membership enforced; options capped at 6; question/option lengths bounded in `canvas/types.rs` |
| Poll close / results | Pass | Pass | Service performs role/membership-aware lookup |
| Poll vote | Pass | Pass | `vote_poll` now enforces server membership before accepting a vote |
| Clip reaction add | Pass | Pass | Service verifies clip server and membership, then enforces reaction cap |
| Clip reaction remove | Pass | Pass | Removal path now enforces `require_member` before deleting any reaction row |
| Pin / unpin clip | Pass | Pass | Admin-only in handlers |
| Event create / update / delete | Pass | Pass | Admin-only; `event_at` must be future and within seven days |
| Hydration / legacy canvas GETs | Pass | Pass | Membership enforced before returning state |
| Canvas WebSocket broadcasts | Pass | N/A | Canvas module emits server-generated `canvas_update` events; no separate unauthenticated inbound canvas WS parser was found |

## Appendix B - E2EE Cryptographic Correctness Verification
| Primitive / Flow | Evidence Reviewed | Result |
|------------------|------------------|--------|
| X3DH | `frontend/src/lib/signal/x3dh.ts` | Uses `@noble/curves` X25519 directly, validates 32-byte public keys, verifies signed prekeys, and explicitly handles OPK-absent fallback without server plaintext access |
| Double Ratchet skipped-key storage | `frontend/src/lib/signal/ratchet.ts` | Bounded with `MAX_SKIPPED_KEYS = 512`; prior unbounded-storage concern is not present in current code |
| Sender-key distribution | `frontend/src/lib/signal/sender_keys.ts` | Uses AES-GCM, Ed25519 signatures, and binds sender identity into ECIES HKDF input to resist key-substitution |
| Media encryption | `frontend/src/lib/signal/mediaEncrypt.ts` | Fresh AES-256 key and 96-bit IV generated per upload with `crypto.getRandomValues()` |
| Recovery-passphrase backup | `frontend/src/lib/signal/backup.ts` | Uses PBKDF2-SHA256 with 1,200,000 iterations, AES-GCM, and passphrase complexity validation |
| Safety numbers | `frontend/src/lib/signal/index.ts:fingerprintKeys` | Deterministic fingerprint over `dhPub || sigPub`; no ordering bug found in current implementation |
| Residual E2EE risk | Regression risk around server-side authorization | No active primitive-level issue confirmed; preserve regression coverage on DM/bootstrap, key-bundle policy, and device-trust promotion paths |

## Appendix C - Compliance Evidence Matrix
| Domain | Evidence Present | Missing / Weak Evidence |
|--------|------------------|-------------------------|
| COPPA | Child accounts, parental tables, pending-friend workflow, parental overview endpoints, shared DM/key-bootstrap policy gate | Preserve regression evidence proving the gate stays applied to both DM creation and key-bundle retrieval |
| GDPR | Broadened export ZIP, wider linked-record cleanup on delete, deletion hold metadata on `users`, documented 30-day retention hold, operational retention cleanup worker, message-content note preserving E2EE | Keep retention docs and basis values aligned if future schema changes introduce new non-cascading references |
| PCI-DSS | Stripe signature verification present; no card-storage path observed in reviewed code | Public compliance docs should explicitly state current Stripe scope and SAQ-A boundary |
| SOC2 CC6 | Strong role checks across most channel/server/canvas flows; trusted-device model exists; reviewed auth/canvas gaps are remediated | Continue regression coverage so the remediated helpers do not drift |
| SOC2 CC7 | Sentry sanitization exists on the frontend; API and WS rate limiting exist; docs-sync CI coverage now guards key security-document invariants | Continue reviewing whether advisory OSV/Trivy scans should stay advisory as scale and audit requirements increase |
| Secrets hygiene | `backend/secrets/` path is absent from git history in this repo snapshot; no committed handover absolute path was found in tracked files reviewed | Continue periodic history scanning and keep this check in CI/pre-release review |
