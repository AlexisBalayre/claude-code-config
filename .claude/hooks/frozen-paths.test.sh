#!/usr/bin/env bash
# Hand-run cases for frozen-paths.sh: `bash .claude/hooks/frozen-paths.test.sh`.
# Each case feeds the hook an Edit/Write payload for a path in a throwaway repo
# and checks the exit code (0 = allowed, 2 = blocked).
set -uo pipefail

HOOK="${HOOK:-$(cd "$(dirname "$0")" && pwd)/frozen-paths.sh}"
TMP=$(mktemp -d)
trap 'chmod -R u+w "$TMP"; rm -r "$TMP"' EXIT
fail=0

repo="$TMP/repo"
git init -q -b feature/allowed "$repo"
mkdir -p "$repo/packages/client/src" "$repo/apps/web" "$repo/apps/api" \
  "$repo/packages/client-x" "$repo/packages/db"
echo x >"$repo/packages/client/src/a.ts"
mkdir "$repo/.claude"
echo "FROZEN_PATHS_REGEX='^(packages/client|apps/web)/'" >"$repo/.claude/project.env"
git -C "$repo" add -A
git -C "$repo" commit -qm init
git -C "$repo" worktree add -q -b feature/other "$TMP/wt-other"
ln -s "$repo" "$TMP/link-to-repo"
mkdir -p "$TMP/wt-other/apps/api"
ln -s ../../packages/client/src/a.ts "$TMP/wt-other/apps/api/alias.ts"

allow="$TMP/frozen-allow.txt"
cat >"$allow" <<'EOF'
# comment line
feature/allowed packages/client/src/a.ts
feature/allowed apps/web/new/b.tsx
EOF

check() { # <name> <want exit> <path> [allow-list path, "-" for unset] [tool, default Edit]
  local name=$1 want=$2 file=$3 list=${4:-$allow} tool=${5:-Edit} key=file_path got payload
  [ "$tool" = NotebookEdit ] && key=notebook_path
  # A here-string, not a pipe: the hook can exit before reading stdin, and jq writing to
  # the closed pipe would fail the case under pipefail (seen on ubuntu CI).
  payload=$(jq -n --arg f "$file" --arg t "$tool" --arg k "$key" '{tool_name: $t, tool_input: ({($k): $f} + (if $t == "NotebookEdit" then {file_path: "/tmp/decoy"} else {} end))}')
  if [ "$list" = - ]; then
    env -u AGENT_FROZEN_ALLOW CLAUDE_PROJECT_DIR="$repo" bash "$HOOK" <<<"$payload" 2>/dev/null
  else
    AGENT_FROZEN_ALLOW=$list CLAUDE_PROJECT_DIR="$repo" bash "$HOOK" <<<"$payload" 2>/dev/null
  fi
  got=$?
  if [ "$got" = "$want" ]; then echo "ok   $name"; else echo "FAIL $name: want $want, got $got"; fail=1; fi
}

check "variable unset: frozen edit passes" 0 "$repo/packages/client/src/z.ts" -
mv "$repo/.claude/project.env" "$TMP/project.env"
check "no FROZEN_PATHS_REGEX: frozen edit passes" 0 "$repo/packages/client/src/z.ts"
mv "$TMP/project.env" "$repo/.claude/project.env"
check "allowed branch and path" 0 "$repo/packages/client/src/a.ts"
check "same path on another branch" 2 "$TMP/wt-other/packages/client/src/a.ts"
check "allowed branch, another frozen file" 2 "$repo/packages/client/src/z.ts"
check "lookalike package outside the freeze" 0 "$repo/packages/client-x/a.ts"
check "unfrozen package" 0 "$repo/packages/db/a.ts"
check "Write into a frozen dir that doesn't exist yet" 2 "$repo/apps/web/missing/c.tsx"
check "allowed new file under a missing dir" 0 "$repo/apps/web/new/b.tsx"
check "variable set, allow-list missing" 2 "$repo/packages/client/src/a.ts" "$TMP/no-such-file"
check "'..' into the frozen app, unlisted" 2 "$repo/apps/api/../web/x.ts"
check "'..' through a missing dir into the frozen app" 2 "$repo/apps/nope/../web/x.ts"
check "symlinked repo dir, allowed file" 0 "$TMP/link-to-repo/packages/client/src/a.ts"
check "symlinked repo dir, unlisted file" 2 "$TMP/link-to-repo/packages/client/src/z.ts"
check "symlinked file pointing into the client, from another branch" 2 "$TMP/wt-other/apps/api/alias.ts"
check "wrong-case path into the frozen app" 2 "$repo/Apps/Web/x.ts"
check "wrong-case frozen dir not yet on disk" 2 "$TMP/wt-other/Apps/Web/x.ts"
check "wrong-case path to an unlisted frozen file" 2 "$repo/Packages/Client/src/z.ts"
if [ -d "$TMP/REPO" ]; then # a case-insensitive filesystem such as APFS
  check "wrong-case repo root into the frozen app" 2 "$TMP/REPO/apps/web/x.ts"
  check "allowed file reached through a wrong-case root" 0 "$TMP/REPO/Packages/Client/src/a.ts"
else
  echo "skip wrong-case repo root: case-sensitive filesystem"
fi
check "newline smuggling an allowed line" 2 "$repo/packages/client/src/z.ts
feature/allowed packages/client/src/a.ts"
check "outside any git tree" 0 "$TMP/loose/packages/client/a.ts"

# The session's cwd must not pick the branch: run from the other worktree.
# check's fail=1 dies with the subshell, so the subshell exits with it.
(cd "$TMP/wt-other" && check "cwd on another branch, file on the allowed one" 0 "$repo/packages/client/src/a.ts" && exit "$fail") || fail=1

msg=$(jq -n --arg f "$repo/apps/api/../web/x.ts" '{tool_name: "Edit", tool_input: {file_path: $f}}' |
  AGENT_FROZEN_ALLOW=$allow CLAUDE_PROJECT_DIR="$repo" bash "$HOOK" 2>&1 >/dev/null)
case "$msg" in
  *"BLOCKED: apps/web/x.ts "*) echo "ok   deny message names the resolved path" ;;
  *) echo "FAIL deny message: $msg"; fail=1 ;;
esac

check "NotebookEdit on an unlisted frozen path" 2 "$repo/apps/web/n.ipynb" "$allow" NotebookEdit
check "NotebookEdit outside the freeze" 0 "$repo/packages/db/n.ipynb" "$allow" NotebookEdit
AGENT_FROZEN_ALLOW=$allow CLAUDE_PROJECT_DIR="$repo" bash "$HOOK" <<<"not json" >/dev/null 2>&1
if [ $? = 2 ]; then echo "ok   payload jq cannot parse"; else echo "FAIL payload jq cannot parse"; fail=1; fi

[ "$fail" = 0 ] && echo "All cases passed" || exit 1
