#!/bin/bash
# Fail when a diff adds a line holding an em or en dash (U+2014, U+2013): new prose,
# comments and copy use a comma, colon or parentheses instead. Only added lines
# count, since older repo prose predates the rule.
#   scripts/check-dashes.sh --cached      ->  the staged diff (scripts/pre-commit)
#   scripts/check-dashes.sh HEAD^1 HEAD   ->  a PR merge commit's diff (CI)
set -euo pipefail

# The dashes are octal UTF-8 bytes so this file passes its own check; the `diff --git`
# flag keeps an added line that starts with "++ " from reading as a file header.
DASHED=$(
  git diff -U0 --no-color --no-ext-diff --no-prefix "$@" \
    | LC_ALL=C awk '
      /^diff --git / { header = 1; next }
      header && /^\+\+\+ / { file = substr($0, 5); header = 0; next }
      /^\+/ && (index($0, "\342\200\224") || index($0, "\342\200\223")) { print "  " file ": " substr($0, 2) }
    '
)
[ -z "$DASHED" ] && exit 0

echo "ERROR: Added lines contain an em or en dash (U+2014, U+2013)." >&2
echo "Use a comma, colon, parentheses or \"to\". A JS, TS or JSON string literal that needs the character can write \\u2014." >&2
echo "$DASHED" >&2
exit 1
