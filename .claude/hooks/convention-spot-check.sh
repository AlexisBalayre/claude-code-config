#!/usr/bin/env bash
# Stop hook: lightweight structural convention spot-check on changed files.
# Advisory only (exit 0). Comment-quality findings are owned by comment-pruner.sh.
set -uo pipefail

# Get changed .ts/.tsx files (staged + unstaged)
CHANGED_FILES=$(git diff --name-only HEAD 2>/dev/null || true)
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null || true)
ALL_FILES=$(echo -e "${CHANGED_FILES}\n${STAGED_FILES}" | sort -u | grep -E '\.(ts|tsx)$' || true)

if [ -z "$ALL_FILES" ]; then
  exit 0
fi

WARNINGS=""

while IFS= read -r file; do
  # Skip if file doesn't exist (deleted files)
  if [ ! -f "$file" ]; then
    continue
  fi

  # Check for export default
  if grep -qE '^\s*export\s+default\s' "$file" 2>/dev/null; then
    WARNINGS+="  ⚠ $file: uses 'export default' — use named exports only\n"
  fi

  # Check for inline type/interface in service or route files
  if [[ "$file" =~ \.(service|routes)\.(ts|tsx)$ ]]; then
    if grep -qE '^\s*export\s+(interface|type)\s' "$file" 2>/dev/null; then
      WARNINGS+="  ⚠ $file: exports type/interface inline — move to types/ folder\n"
    fi
  fi

  # Skip JSDoc check in tests/fixtures/mocks — these exports don't need docs.
  if [[ "$file" =~ \.(test|spec|mock|fixture)\.(ts|tsx)$ ]] || [[ "$file" =~ /__mocks__/ ]] || [[ "$file" =~ /test/ ]]; then
    continue
  fi

  # JSDoc on exports is only required for the package public surface (packages/*).
  # apps/* and services/* are leaf workspaces with no external consumers; JSDoc
  # there is optional and only added when WHY is non-obvious. The path-scoped
  # checks further down (Readonly<Props> for apps, EventEmitter ban for
  # session-engine, factory-only services for gateway) still apply, so we wrap
  # only the JSDoc check in the packages/* guard rather than skipping the rest
  # of the iteration.
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
      WARNINGS+="  ⚠ $file: $COUNT export(s) may be missing JSDoc comments\n"
    fi
  fi

  # Frontend: React component props should be Readonly<Props>.
  # Flags destructured or positional props typed as `XProps` without Readonly wrapper.
  if [[ "$file" == apps/acme-web/* ]] && [[ "$file" =~ \.tsx$ ]]; then
    NON_READONLY_PROPS=$(grep -nE ':\s*\w+Props[[:space:]]*[,)=]' "$file" 2>/dev/null | grep -vE ':\s*Readonly<' || true)
    if [ -n "$NON_READONLY_PROPS" ]; then
      WARNINGS+="  ⚠ $file: component props not wrapped in Readonly<…> (see docs/conventions/core.md)\n"
    fi
  fi

  # Session engine: EventEmitter is forbidden (see docs/conventions/services.md — "NEVER EventEmitter").
  if [[ "$file" == services/acme-session-engine/* ]]; then
    if grep -qE "from\s+['\"](node:events|events)['\"]" "$file" 2>/dev/null; then
      WARNINGS+="  ⚠ $file: imports EventEmitter — use typed callbacks instead (docs/conventions/services.md)\n"
    fi
  fi

  # Gateway: services must be created via factories, never via `new XService()`
  # (see docs/conventions/services.md — "factories ONLY, NEVER classes").
  if [[ "$file" == services/acme-gateway/* ]]; then
    if grep -qE 'new\s+[A-Z][A-Za-z0-9_]*Service\s*\(' "$file" 2>/dev/null; then
      WARNINGS+="  ⚠ $file: instantiates a Service class directly — use the create*Service factory (docs/conventions/services.md)\n"
    fi
  fi

done <<< "$ALL_FILES"

if [ -n "$WARNINGS" ]; then
  echo "" >&2
  echo "Convention spot-check warnings:" >&2
  echo -e "$WARNINGS" >&2
  echo "These are advisory — fix before committing if possible." >&2
fi

exit 0
