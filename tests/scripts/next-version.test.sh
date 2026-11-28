#!/usr/bin/env bash
# Hand-run cases for scripts/next-version.sh: `bash tests/scripts/next-version.test.sh`.
# Each case commits subjects on top of a tag in a throwaway repo and checks the tag printed.

set -uo pipefail

NEXT="$(cd "$(dirname "$0")/../../scripts" && pwd)/next-version.sh"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0

# The fixtures must not inherit the user's hooksPath or commit signing.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

n=0
expect() { # <want> <name> <last tag, "" for none> <commit message>...
  local want=$1 name=$2 tag=$3 got message
  shift 3
  n=$((n + 1))
  git init -q -b main "$T/$n"
  cd "$T/$n" || exit 1
  git commit -q --allow-empty -m "feat: before the tag"
  [ -z "$tag" ] || git tag "$tag"
  for message in "$@"; do git commit -q --allow-empty -m "$message"; done
  got=$(bash "$NEXT" 2>/dev/null) || got="exit $?"
  if [ "$got" != "$want" ]; then
    printf 'FAIL want=%s got=%s  %s\n' "${want:-<none>}" "${got:-<none>}" "$name"
    failures=$((failures + 1))
  fi
}

expect v1.1.1 'fix bumps the patch' v1.1.0 'fix(ci): pin the runner (#75)'
expect v1.1.1 'perf bumps the patch' v1.1.0 'perf: skip the scan'
expect v1.2.0 'feat bumps the minor and resets the patch' v1.1.4 'feat(review): add a panel (#76)'
expect v1.2.0 'the highest bump in the range wins' v1.1.0 'feat: a' 'fix: b' 'docs: c'
expect v1.2.0 'an older fix does not lower a newer feat' v1.1.0 'fix: a' 'feat: b'
expect v2.0.0 'an older feat does not lower a newer breaking commit' v1.1.0 'feat: a' 'fix: b

BREAKING CHANGE: x'
expect v2.0.0 'a bang subject bumps the major' v1.4.2 'feat(skills)!: drop a skill' 'feat: a'
expect v2.0.0 'a bang on any type bumps the major' v1.4.2 'refactor!: move the hooks'
expect v2.0.0 'a BREAKING CHANGE footer bumps the major' v1.4.2 'fix: tighten the gate

BREAKING CHANGE: the gate now refuses an unstamped tree.'
expect v2.0.0 'a BREAKING-CHANGE footer bumps the major' v1.4.2 'fix: a

BREAKING-CHANGE: x'
expect '' 'docs, chore, test and refactor release nothing' v1.1.0 'docs: a' 'chore(ci): bump x' 'test: b' 'refactor(tests): c'
expect '' 'no commit since the tag' v1.1.0
expect '' 'a body line shaped like a subject is not a bump' v1.1.0 'docs: quote a title

feat: this is prose in a body'
expect '' 'a type that only starts with feat' v1.1.0 'feature: not a type'
expect v0.1.0 'no tag yet: counted from 0.0.0 over all history' ''
expect v1.1.1 'a commit no release covers yet' v1.1.0 'fix: a' && {
  cd "$T/$n" && git tag nightly && git commit -q --allow-empty -m 'docs: b'
  got=$(bash "$NEXT")
  [ "$got" = v1.1.1 ] || { echo "FAIL want=v1.1.1 got=$got  a tag that is not a release is skipped"; failures=$((failures + 1)); }
  git commit -q --allow-empty -m 'feat: c' && git tag v1.2.0 && git switch -q --detach HEAD~1
  got=$(bash "$NEXT")
  [ -z "$got" ] || { echo "FAIL want=<none> got=$got  a stale commit under a later release"; failures=$((failures + 1)); }
}
expect 'exit 1' 'a v tag with a suffix stops the release' v1.1.0-rc.1 'fix: a'

[ "$failures" -eq 0 ] && echo "next-version: all cases pass" || exit 1
