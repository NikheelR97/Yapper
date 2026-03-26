#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

base_sha="${BASE_SHA:-}"
if [[ -z "$base_sha" ]]; then
  base_sha="$(git rev-parse HEAD~1 2>/dev/null || true)"
fi

if [[ -z "$base_sha" ]]; then
  echo "Unable to determine a diff base SHA for the /api/v1 guard." >&2
  exit 1
fi

mapfile -d '' changed_files < <(
  git diff --name-only -z --diff-filter=ACMRT "$base_sha" HEAD -- \
    backend/src \
    backend/tests \
    frontend/src \
    frontend/tests \
    frontend/src-tauri \
    wiki \
    wiki-repo \
    ui-specs \
    .github/workflows \
    .github/scripts \
    docs/api.md \
    docs/architecture.md \
    docs/e2ee.md \
    docs/deployment.md \
    "dev docs/HANDOVER.md"
)

if [[ ${#changed_files[@]} -eq 0 ]]; then
  exit 0
fi

violations=()
for file in "${changed_files[@]}"; do
  [[ -f "$file" ]] || continue
  if [[ "$file" == ".github/scripts/check-no-v1-http-routes.sh" ]]; then
    continue
  fi
  if [[ "$file" == *"backend/tests/integration/v1_absence.rs" ]]; then
    continue
  fi
  if rg -n '/api/v1' "$file" >/dev/null; then
    violations+=("$file")
  fi
done

if [[ ${#violations[@]} -eq 0 ]]; then
  exit 0
fi

echo "Found forbidden /api/v1 HTTP route references in changed files:" >&2
for file in "${violations[@]}"; do
  echo "  - $file" >&2
  rg -n '/api/v1' "$file" >&2
done

echo "Use /api/v2 for HTTP routes. Keep only intentional non-HTTP literals such as cryptographic _v1 info strings." >&2
exit 1
