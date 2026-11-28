#!/bin/bash
# Print the next release tag for HEAD, or nothing when no commit since the last
# release warrants one. Reads the Conventional Commit subjects and bodies since
# the newest vX.Y.Z tag reachable from HEAD (all of history when there is none):
#   `type!:` subject or a `BREAKING CHANGE:` footer  ->  major
#   feat                                             ->  minor
#   fix, perf                                        ->  patch
# Called by .github/workflows/release.yml; prints for example `v1.2.0`.
set -euo pipefail

# A re-run of an old workflow run sits on a commit a later release already shipped.
[ -z "$(git tag --contains HEAD --list 'v[0-9]*.[0-9]*.[0-9]*')" ] || exit 0

last=$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*' 2>/dev/null || true)
if [ -n "$last" ] && ! [[ $last =~ ^v([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "ERROR: the last release tag '$last' is not vX.Y.Z." >&2
  exit 1
fi
major=${BASH_REMATCH[1]:-0} minor=${BASH_REMATCH[2]:-0} patch=${BASH_REMATCH[3]:-0}

# %x1e ends each subject so a body line shaped like a subject cannot bump the version.
bump=$(
  git log --format='%s%x1e%n%b' "${last:+$last..}HEAD" | awk '
    index($0, "\036") {
      if ($0 ~ /^[a-z]+(\([^)]*\))?!:/) level = 3
      else if ($0 ~ /^feat(\([^)]*\))?:/ && level < 2) level = 2
      else if ($0 ~ /^(fix|perf)(\([^)]*\))?:/ && level < 1) level = 1
      next
    }
    /^BREAKING[ -]CHANGE:/ { level = 3 }
    END { print level + 0 }
  '
)

case $bump in
  3) echo "v$((major + 1)).0.0" ;;
  2) echo "v$major.$((minor + 1)).0" ;;
  1) echo "v$major.$minor.$((patch + 1))" ;;
esac
