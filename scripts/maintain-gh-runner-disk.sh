#!/usr/bin/env bash
#
# Maintain disk usage on self-hosted GitHub Actions runners.
#
# Safe default:
#   - dry-run mode unless --apply is passed
#   - skips runner workspace cleanup while a Runner.Worker process is active
#   - keeps the newest Playwright browser builds per browser family
#
# Typical use on the runner VM:
#   bash scripts/maintain-gh-runner-disk.sh --apply
#
# Cron example:
#   17 3 * * * /bin/bash /home/runner/Yapper/scripts/maintain-gh-runner-disk.sh --apply >> /home/runner/runner-maintenance.log 2>&1

set -euo pipefail

DRY_RUN=1
FORCE=0
DOCKER_PRUNE=0
MAX_AGE_DAYS="${MAX_AGE_DAYS:-14}"
KEEP_PLAYWRIGHT_BUILDS="${KEEP_PLAYWRIGHT_BUILDS:-2}"
RUNNER_ROOT="${RUNNER_ROOT:-$HOME}"
PLAYWRIGHT_CACHE="${PLAYWRIGHT_CACHE:-$HOME/.cache/ms-playwright}"

usage() {
	cat <<'EOF'
Usage: maintain-gh-runner-disk.sh [options]

Options:
  --apply                         Delete matching files. Default is dry-run.
  --dry-run                       Print what would be deleted.
  --force                         Allow workspace cleanup even if Runner.Worker is active.
  --docker-prune                  Also run docker system prune for old build cache/images.
  --max-age-days N                Delete stale artifacts older than N days. Default: 14.
  --keep-playwright-builds N      Keep newest N builds per Playwright browser family. Default: 2.
  --runner-root DIR               Root containing actions-runner-* directories. Default: $HOME.
  --playwright-cache DIR          Playwright browser cache. Default: ~/.cache/ms-playwright.
  -h, --help                      Show this help.

Environment overrides:
  MAX_AGE_DAYS, KEEP_PLAYWRIGHT_BUILDS, RUNNER_ROOT, PLAYWRIGHT_CACHE
EOF
}

log() { printf '%s\n' "$*"; }
section() { printf '\n==> %s\n' "$*"; }

delete_path() {
	local target="$1"
	if [[ ! -e "$target" ]]; then
		return
	fi

	if [[ "$DRY_RUN" -eq 1 ]]; then
		printf '[dry-run] rm -rf %q\n' "$target"
	else
		rm -rf -- "$target"
	fi
}

delete_find_matches() {
	local root="$1"
	shift

	if [[ ! -d "$root" ]]; then
		return
	fi

	while IFS= read -r -d '' path; do
		delete_path "$path"
	done < <(find "$root" "$@" -print0 2>/dev/null || true)
}

active_runner_worker() {
	pgrep -f 'Runner\.Worker' >/dev/null 2>&1
}

human_du() {
	local path="$1"
	if [[ -e "$path" ]]; then
		du -sh "$path" 2>/dev/null || true
	fi
}

parse_args() {
	while [[ $# -gt 0 ]]; do
		case "$1" in
			--apply)
				DRY_RUN=0
				;;
			--dry-run)
				DRY_RUN=1
				;;
			--force)
				FORCE=1
				;;
			--docker-prune)
				DOCKER_PRUNE=1
				;;
			--max-age-days)
				MAX_AGE_DAYS="${2:?Missing value for --max-age-days}"
				shift
				;;
			--keep-playwright-builds)
				KEEP_PLAYWRIGHT_BUILDS="${2:?Missing value for --keep-playwright-builds}"
				shift
				;;
			--runner-root)
				RUNNER_ROOT="${2:?Missing value for --runner-root}"
				shift
				;;
			--playwright-cache)
				PLAYWRIGHT_CACHE="${2:?Missing value for --playwright-cache}"
				shift
				;;
			-h|--help)
				usage
				exit 0
				;;
			*)
				printf 'Unknown option: %s\n\n' "$1" >&2
				usage >&2
				exit 2
				;;
		esac
		shift
	done
}

validate_number() {
	local name="$1"
	local value="$2"
	if ! [[ "$value" =~ ^[0-9]+$ ]]; then
		printf '%s must be a non-negative integer, got: %s\n' "$name" "$value" >&2
		exit 2
	fi
}

clean_playwright_cache() {
	section "Playwright browser cache"
	log "Cache: $PLAYWRIGHT_CACHE"
	human_du "$PLAYWRIGHT_CACHE"

	if [[ ! -d "$PLAYWRIGHT_CACHE" ]]; then
		log "No Playwright cache found."
		return
	fi

	local family
	for family in chromium chrome ffmpeg firefox webkit; do
		mapfile -t builds < <(
			find "$PLAYWRIGHT_CACHE" -mindepth 1 -maxdepth 1 -type d -name "${family}-*" -printf '%T@ %p\n' 2>/dev/null |
				sort -rn |
				awk '{ $1=""; sub(/^ /, ""); print }'
		)

		if [[ "${#builds[@]}" -le "$KEEP_PLAYWRIGHT_BUILDS" ]]; then
			continue
		fi

		local index=0
		local build
		for build in "${builds[@]}"; do
			index=$((index + 1))
			if [[ "$index" -le "$KEEP_PLAYWRIGHT_BUILDS" ]]; then
				continue
			fi

			if find "$build" -maxdepth 0 -mtime +"$MAX_AGE_DAYS" -print -quit | grep -q .; then
				delete_path "$build"
			fi
		done
	done
}

clean_runner_workspaces() {
	section "Runner workspaces"

	if active_runner_worker && [[ "$FORCE" -ne 1 ]]; then
		log "Runner process is active; skipping workspace cleanup. Re-run with --force during a maintenance window if needed."
		return
	fi

	local runner_dir
	while IFS= read -r -d '' runner_dir; do
		log "Runner: $runner_dir"
		human_du "$runner_dir/_work"

		delete_find_matches "$runner_dir/_work/_temp" -mindepth 1 -mtime +"$MAX_AGE_DAYS"
		delete_find_matches "$runner_dir/_work" -type d \( \
			-name test-results -o \
			-name playwright-report -o \
			-name allure-results -o \
			-name allure-report \
		\) -mtime +"$MAX_AGE_DAYS"
	done < <(find "$RUNNER_ROOT" -maxdepth 1 -type d -name 'actions-runner*' -print0 2>/dev/null || true)
}

clean_user_caches() {
	section "User cache artifacts"
	human_du "$HOME/.npm"

	delete_find_matches "$HOME/.npm/_cacache/tmp" -mindepth 1 -mtime +"$MAX_AGE_DAYS"
	delete_find_matches "$HOME/.cache" -type d \( \
		-name playwright-report -o \
		-name test-results \
	\) -mtime +"$MAX_AGE_DAYS"
}

prune_docker() {
	if [[ "$DOCKER_PRUNE" -ne 1 ]]; then
		return
	fi

	section "Docker prune"
	if ! command -v docker >/dev/null 2>&1; then
		log "Docker not found; skipping."
		return
	fi

	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "[dry-run] docker system prune --force --filter until=${MAX_AGE_DAYS}d"
	else
		docker system prune --force --filter "until=${MAX_AGE_DAYS}d"
	fi
}

main() {
	parse_args "$@"
	validate_number "--max-age-days" "$MAX_AGE_DAYS"
	validate_number "--keep-playwright-builds" "$KEEP_PLAYWRIGHT_BUILDS"

	section "Disk before cleanup"
	df -h "$RUNNER_ROOT" "$HOME" 2>/dev/null || df -h

	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "Mode: dry-run. Pass --apply to delete files."
	else
		log "Mode: apply."
	fi

	clean_playwright_cache
	clean_runner_workspaces
	clean_user_caches
	prune_docker

	section "Disk after cleanup"
	df -h "$RUNNER_ROOT" "$HOME" 2>/dev/null || df -h
}

main "$@"
