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

# All checks are advisory (warnings only) because the wiki is a separate
# repo that PRs cannot update.  The check surfaces drift as annotations.

if grep -Fq 'ws?token=' "$api_doc"; then
  echo "::warning::API reference still documents WebSocket query-string tokens — update the wiki"
fi

if ! grep -Fq 'wss://api.yapperhq.com/ws' "$api_doc"; then
  echo "::warning::API reference is missing WebSocket URL documentation"
fi

if ! grep -Fq '/api/v2/notifications/push-token' "$api_doc"; then
  echo "::warning::API reference is missing push-token endpoint documentation"
fi

if ! grep -Fq 'Nine routes are explicitly CSRF-exempt' "$security_doc"; then
  echo "::warning::Security doc is missing CSRF exemption list"
fi

if ! grep -Fq '/support/webhooks/hubspot' "$security_doc"; then
  echo "::warning::Security doc is missing HubSpot webhook route"
fi

if ! grep -Fq '/auth/oauth/exchange' "$security_doc"; then
  echo "::warning::Security doc is missing OAuth exchange route"
fi
