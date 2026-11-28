#!/usr/bin/env bash
# Fails when a conventions doc and its rule files drift: `bash scripts/conventions-rules.test.sh`.
# Each docs/conventions/<area>.md with `paths:` needs .claude/rules/<area>-conventions.md as a
# symlink to it: without one, Claude Code never injects the doc.
set -euo pipefail

cd "$(dirname "$0")/.."
failures=0
checked=0
fail() {
  echo "conventions-rules: $1" >&2
  failures=$((failures + 1))
}

for doc in docs/conventions/*.md; do
  area=$(basename "$doc" .md)
  rule=.claude/rules/$area-conventions.md
  paths=$(awk 'NR == 1 { next } /^---$/ { exit } /^[[:space:]]*- / { sub(/^[[:space:]]*- "?/, ""); sub(/"$/, ""); print }' "$doc" | paste -sd , -)
  if [ -z "$paths" ]; then
    if [ -e "$rule" ] || [ -L "$rule" ]; then fail "$doc has no paths:, so $rule must not exist"; fi
    continue
  fi
  checked=$((checked + 1))
  [ "$(readlink "$rule" 2>/dev/null || true)" = "../../docs/conventions/$area.md" ] ||
    fail "$rule must be a symlink to ../../docs/conventions/$area.md"
done

for file in .claude/rules/*-conventions.md; do
  [ -e "$file" ] || [ -L "$file" ] || continue
  area=$(basename "$file")
  area=${area%-conventions.*}
  [ -f "docs/conventions/$area.md" ] || fail "$file names no docs/conventions/$area.md"
done

[ "$failures" -eq 0 ] || exit 1
echo "conventions-rules: $checked conventions docs, each with a matching .claude rule"
