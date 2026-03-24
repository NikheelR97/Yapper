#!/usr/bin/env bash
set -euo pipefail

# Clone the wiki repo if not already present
if [ ! -d "wiki-repo" ]; then
  git clone "https://github.com/${GITHUB_REPOSITORY}.wiki.git" wiki-repo 2>/dev/null || {
    echo "Wiki not available — skipping docs-sync check"
    exit 0
  }
fi

api_doc="wiki-repo/API-Reference.md"
security_doc="wiki-repo/Security.md"

if [ ! -f "$api_doc" ] || [ ! -f "$security_doc" ]; then
  echo "Wiki docs missing — skipping docs-sync check"
  exit 0
fi

# --- Negative checks: fail if OUTDATED content is still present ---

if grep -Fq 'ws?token=' "$api_doc"; then
  echo "API reference still documents WebSocket query-string tokens"
  exit 1
fi

if grep -Fq '/api/v1/notifications/device-token' "$api_doc"; then
  echo "API reference still documents the legacy device-token route"
  exit 1
fi

# --- Positive checks: warn (don't fail) if expected content is missing ---
# These validate that the wiki has been updated to reflect current implementation.
# They are advisory until the wiki is fully populated.

warn=0
if ! grep -Fq 'wss://api.yapperhq.com/ws' "$api_doc"; then
  echo "::warning::API reference is missing WebSocket URL documentation"
  warn=1
fi
if ! grep -Fq '/api/v1/notifications/push-token' "$api_doc"; then
  echo "::warning::API reference is missing push-token endpoint documentation"
  warn=1
fi
if ! grep -Fq 'Nine routes are explicitly CSRF-exempt' "$security_doc"; then
  echo "::warning::Security doc is missing CSRF exemption list"
  warn=1
fi
if ! grep -Fq '/support/webhooks/hubspot' "$security_doc"; then
  echo "::warning::Security doc is missing HubSpot webhook route"
  warn=1
fi
if ! grep -Fq '/auth/oauth/exchange' "$security_doc"; then
  echo "::warning::Security doc is missing OAuth exchange route"
  warn=1
fi

if [ "$warn" -gt 0 ]; then
  echo "Some wiki docs are incomplete — see warnings above"
fi
