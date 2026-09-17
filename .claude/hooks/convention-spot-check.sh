#!/usr/bin/env bash
# Stop hook: structural convention spot-check on the files this session changed.
# Findings reach the model once per Stop cycle (exit 2); on the stop_hook_active
# re-run the hook is silent so a heuristic the model judged a false positive
# cannot loop. Exit-0 output never reaches the model, so "advisory" here means
# "shown once, then dropped". Named exports are Biome's job (noDefaultExport, run
# by quality-checks.sh); comment quality is comment-pruner.sh's.
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}" 2>/dev/null || exit 0

if [ ! -t 0 ]; then
  INPUT=$(cat 2>/dev/null || true)
  printf '%s' "$INPUT" | grep -q '"stop_hook_active"[[:space:]]*:[[:space:]]*true' && exit 0
fi

# Untracked files included: a freshly written file is where a wrong role or an
# inline type most often lands.
ALL_FILES=$(
  {
    git diff --name-only HEAD 2>/dev/null
    git diff --cached --name-only 2>/dev/null
    git ls-files --others --exclude-standard 2>/dev/null
  } | grep -E '\.(ts|tsx)$' | sort -u
)
[ -z "$ALL_FILES" ] && exit 0

WARNINGS=""

while IFS= read -r file; do
  [ -f "$file" ] || continue

  if [[ "$file" =~ \.(service|routes)\.(ts|tsx)$ ]]; then
    if grep -qE '^\s*export\s+(interface|type)\s' "$file" 2>/dev/null; then
      WARNINGS+="  $file: exports a type/interface inline; move it to types/ (core.md §Structure)\n"
    fi
  fi

  if [[ "$file" =~ \.(test|spec|mock|fixture)\.(ts|tsx)$ ]] || [[ "$file" =~ /__mocks__/ ]] || [[ "$file" =~ /test/ ]]; then
    continue
  fi

  # JSDoc is required only on the package public surface (core.md §JSDoc).
  if [[ "$file" == packages/* ]]; then
    EXPORTS_WITHOUT_JSDOC=$(awk '
      BEGIN { saw_close = 0 }
      /\*\/[[:space:]]*$/ { saw_close = 1; next }
      /^[[:space:]]*export[[:space:]]+(function|class|const|interface|type|enum)[[:space:]]/ {
        if (!saw_close) print NR": "$0
        saw_close = 0
      }
    ' "$file" 2>/dev/null || true)
    if [ -n "$EXPORTS_WITHOUT_JSDOC" ]; then
      COUNT=$(echo "$EXPORTS_WITHOUT_JSDOC" | wc -l | tr -d ' ')
      WARNINGS+="  $file: $COUNT export(s) may be missing JSDoc (core.md §JSDoc)\n"
    fi
  fi

  if [[ "$file" == apps/acme-web/* ]] && [[ "$file" =~ \.tsx$ ]]; then
    NON_READONLY_PROPS=$(grep -nE ':\s*\w+Props[[:space:]]*[,)=]' "$file" 2>/dev/null | grep -vE ':\s*Readonly<' || true)
    if [ -n "$NON_READONLY_PROPS" ]; then
      WARNINGS+="  $file: component props not wrapped in Readonly<...> (core.md §TypeScript)\n"
    fi
  fi

  if [[ "$file" == services/acme-session-engine/* ]]; then
    if grep -qE "from\s+['\"](node:events|events)['\"]" "$file" 2>/dev/null; then
      WARNINGS+="  $file: imports EventEmitter; use typed callback arrays (services.md §Session Engine › Events & errors)\n"
    fi
  fi

  if [[ "$file" == services/acme-gateway/* ]]; then
    if grep -qE 'new\s+[A-Z][A-Za-z0-9_]*Service\s*\(' "$file" 2>/dev/null; then
      WARNINGS+="  $file: instantiates a Service class directly; use the create*Service factory (services.md §Gateway › Layout)\n"
    fi
  fi
done <<< "$ALL_FILES"

[ -z "$WARNINGS" ] && exit 0

{
  echo
  echo "Convention spot-check on files changed this session:"
  echo -e "$WARNINGS"
  echo "Fix each real finding; if one is a heuristic misfire, say so and stop again (this check stays silent on the re-run)."
} >&2
exit 2
