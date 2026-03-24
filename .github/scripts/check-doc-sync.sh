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

grep -Fq 'wss://api.yapperhq.com/ws' "$api_doc"
if grep -Fq 'ws?token=' "$api_doc"; then
  echo "API reference still documents WebSocket query-string tokens"
  exit 1
fi

grep -Fq '/api/v1/notifications/push-token' "$api_doc"
if grep -Fq '/api/v1/notifications/device-token' "$api_doc"; then
  echo "API reference still documents the legacy device-token route"
  exit 1
fi

grep -Fq 'Nine routes are explicitly CSRF-exempt' "$security_doc"
grep -Fq '/support/webhooks/hubspot' "$security_doc"
grep -Fq '/auth/oauth/exchange' "$security_doc"
