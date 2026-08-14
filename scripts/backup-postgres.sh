#!/usr/bin/env bash
#
# Yapper — encrypted PostgreSQL backup (Coolify homelab host).
#
# WHY ENCRYPTED: the dump contains Signal identity keys + prekeys, Argon2 password
# hashes, user emails, and COPPA-protected child dates of birth. R2's server-side
# encryption is not sufficient — Cloudflare would hold the key. We encrypt with `age`
# BEFORE upload so the storage provider only ever sees ciphertext.
#
# The AGE_RECIPIENT below is a PUBLIC key. The matching PRIVATE key is the recovery
# key: store it OFF this machine (password manager). If the box dies and the private
# key died with it, the backups are permanently unreadable.
#
# Remote retention is handled by an R2 bucket lifecycle rule, not by this script
# (ponytail: the storage layer already does expiry, no reason to reimplement it).
# This script prunes only the local copies.
#
# Usage: ./backup-postgres.sh          (intended to run from cron/Coolify scheduled task)

set -euo pipefail

# ─── Required config ──────────────────────────────────────────────────────────
: "${BACKUP_DATABASE_URL:?BACKUP_DATABASE_URL is required (postgres://user:pass@host:5432/yapper)}"
: "${AGE_RECIPIENT:?AGE_RECIPIENT is required (age public key, e.g. age1...)}"
: "${R2_ACCOUNT_ID:?R2_ACCOUNT_ID is required}"
: "${R2_BACKUP_BUCKET:?R2_BACKUP_BUCKET is required (MUST be a different bucket than media)}"
: "${AWS_ACCESS_KEY_ID:?AWS_ACCESS_KEY_ID is required (R2 token scoped to the BACKUP bucket only)}"
: "${AWS_SECRET_ACCESS_KEY:?AWS_SECRET_ACCESS_KEY is required}"

# ─── Optional config ──────────────────────────────────────────────────────────
BACKUP_DIR="${BACKUP_DIR:-/var/backups/yapper}"
KEEP_LOCAL_DAYS="${KEEP_LOCAL_DAYS:-7}"
R2_ENDPOINT="https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com"

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
# Sunday dumps also land under weekly/ so lifecycle can keep them longer.
if [ "$(date -u +%u)" -eq 7 ]; then prefix="weekly"; else prefix="daily"; fi
filename="yapper-${timestamp}.sql.zst.age"
local_path="${BACKUP_DIR}/${filename}"

mkdir -p "$BACKUP_DIR"

echo "[backup] dumping -> ${local_path}"
# pipefail (set above) makes a pg_dump/zstd failure fail the whole pipeline rather
# than silently producing a truncated, well-formed .age file.
pg_dump --no-owner --no-privileges "$BACKUP_DATABASE_URL" \
  | zstd -q -T0 \
  | age -r "$AGE_RECIPIENT" -o "$local_path"

# A backup that exists but is empty is worse than no backup — it looks like success.
if [ ! -s "$local_path" ]; then
  echo "[backup] FATAL: produced an empty file" >&2
  rm -f "$local_path"
  exit 1
fi
echo "[backup] wrote $(du -h "$local_path" | cut -f1)"

echo "[backup] uploading -> s3://${R2_BACKUP_BUCKET}/${prefix}/${filename}"
aws s3 cp "$local_path" "s3://${R2_BACKUP_BUCKET}/${prefix}/${filename}" \
  --endpoint-url "$R2_ENDPOINT" \
  --only-show-errors

echo "[backup] pruning local copies older than ${KEEP_LOCAL_DAYS} days"
find "$BACKUP_DIR" -name 'yapper-*.sql.zst.age' -type f -mtime "+${KEEP_LOCAL_DAYS}" -delete

echo "[backup] OK ${filename}"
