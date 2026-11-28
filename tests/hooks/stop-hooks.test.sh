#!/usr/bin/env bash
# Hand-run cases for the Stop hook: `bash tests/hooks/stop-hooks.test.sh`.
# Each fixture tree's .claude/project.env names a stub command that records where it ran
# and with what, so the cases check which tree quality-checks.sh works on, and that it
# never runs the typecheck.
set -uo pipefail

DIR="${HOOKS_DIR:-$(cd "$(dirname "$0")/../../.claude/hooks" && pwd)}"
TMP=$(mktemp -d)
trap 'chmod -R u+w "$TMP"; rm -r "$TMP"' EXIT
fail=0

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

profile() {
  mkdir -p "$1/.claude"
  printf 'SOURCE_EXTENSIONS="ts"\nFORMAT_FIX_CMD="stub fix"\nLINT_CMD="stub lint"\nTYPECHECK_CMD="stub typecheck"\n' >"$1/.claude/project.env"
}

main="$TMP/main"
git init -q -b main "$main"
echo 'export const a = 1;' >"$main/a.ts"
profile "$main"
git -C "$main" add -A
git -C "$main" commit -qm init
git -C "$main" worktree add -q -b feature/x "$TMP/wt"

mkdir "$TMP/bin"
cat >"$TMP/bin/stub" <<'EOF'
#!/usr/bin/env bash
echo "$(pwd -P) stub $*" >>"$LOG"
[ "${STUB_FAIL:-}" = "$1" ] && { seq 200; exit 1; }
exit 0
EOF
chmod +x "$TMP/bin/stub"
export PATH="$TMP/bin:$PATH" LOG="$TMP/calls.log" CLAUDE_PROJECT_DIR="$main"

ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }

run_qc() { # <cwd or ""> -> exit code; output in $TMP/out
  : >"$LOG"
  if [ -n "$1" ]; then jq -n --arg c "$1" '{cwd: $c, stop_hook_active: false}'; else echo '{}'; fi |
    bash "$DIR/quality-checks.sh" >"$TMP/out" 2>&1
}

echo 'export const b = 2;' >"$TMP/wt/b.ts"
real_wt=$(cd -P "$TMP/wt" && pwd -P)
real_main=$(cd -P "$main" && pwd -P)

run_qc "$TMP/wt"
grep -qx "$real_wt stub lint b.ts" "$LOG" && ok "worktree cwd: lint runs in the worktree, on its dirty file" || bad "worktree cwd: lint not in the worktree: $(cat "$LOG")"
grep -qx "$real_wt stub fix b.ts" "$LOG" && ok "worktree cwd: so does the formatter" || bad "worktree cwd: no formatter: $(cat "$LOG")"
grep -q " stub typecheck" "$LOG" && bad "worktree cwd: typecheck ran: $(cat "$LOG")" || ok "worktree cwd: no typecheck"

echo 'export const c = 3;' >"$main/c.ts"
run_qc "$TMP/bin"
grep -q "^$real_main stub lint" "$LOG" && ok "cwd outside a git tree falls back to the main checkout" || bad "no-git cwd: $(cat "$LOG")"

sib="$TMP/sibling"
git init -q -b main "$sib"
echo 'export const s = 1;' >"$sib/s.ts"
profile "$sib"
real_sib=$(cd -P "$sib" && pwd -P)
run_qc "$sib"
grep -q "^$real_sib " "$LOG" && bad "sibling repo cwd: lint ran there: $(cat "$LOG")" ||
  { grep -q "^$real_main stub lint" "$LOG" && ok "sibling repo cwd: falls back to the main checkout" || bad "sibling repo cwd: $(cat "$LOG")"; }

run_qc "$main"
grep -q "^$real_main stub lint" "$LOG" && ! grep -q " stub typecheck" "$LOG" &&
  ok "main cwd: lint runs in main, no typecheck" || bad "main cwd: $(cat "$LOG")"

spaced="$TMP/my main"
git init -q -b main "$spaced"
echo 'export const d = 4;' >"$spaced/d.ts"
profile "$spaced"
real_spaced=$(cd -P "$spaced" && pwd -P)
CLAUDE_PROJECT_DIR="$spaced" run_qc "$spaced"
grep -q "^$real_spaced stub lint" "$LOG" && ok "main checkout path with a space: lint runs" || bad "spaced main: $(cat "$LOG")"

# git before 2.31 prints --path-format back and exits 0; a sibling cwd at its
# top level must still fall back.
mkdir "$TMP/oldgit"
printf '#!/usr/bin/env bash\nargs=()\nfor a in "$@"; do case $a in --path-format*) echo "$a" ;; *) args+=("$a") ;; esac; done\nexec %q "${args[@]}"\n' "$(command -v git)" >"$TMP/oldgit/git"
chmod +x "$TMP/oldgit/git"
PATH="$TMP/oldgit:$PATH" run_qc "$sib"
grep -q "^$real_sib " "$LOG" && bad "old git, sibling cwd: lint ran there" ||
  { grep -q "^$real_main stub lint" "$LOG" && ok "old git, sibling cwd: falls back to the main checkout" || bad "old git, sibling cwd: $(cat "$LOG")"; }

run_qc ""
grep -q "^$real_main stub lint" "$LOG" && ok "no cwd: falls back to \$CLAUDE_PROJECT_DIR" || bad "no cwd: $(cat "$LOG")"

STUB_FAIL=lint run_qc "$TMP/wt"
code=$?
[ "$code" = 2 ] && [ "$(grep -cE '^[0-9]+$' "$TMP/out")" -le 40 ] &&
  ok "failing lint in a worktree: exit 2, tailed" || bad "failing lint: exit $code"

STUB_FAIL=fix run_qc "$TMP/wt"
code=$?
[ "$code" = 0 ] && [ ! -s "$TMP/out" ] && ok "a failing formatter is silent: the lint reports what is left" || bad "failing formatter: exit $code: $(cat "$TMP/out")"

rm "$TMP/wt/b.ts"
run_qc "$TMP/wt"
[ ! -s "$LOG" ] && ok "no dirty source file: nothing runs" || bad "clean worktree: $(cat "$LOG")"

[ "$fail" = 0 ] && echo "All cases passed" || exit 1
