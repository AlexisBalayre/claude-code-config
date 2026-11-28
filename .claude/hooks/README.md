# Hook catalog

Hooks are deterministic shell scripts wired in [`settings.json`](../settings.json) that run on
lifecycle events — zero LLM cost, zero context, no hallucination. You never invoke them; they
fire automatically. Exit code `2` blocks the operation, `0` allows it.

Stack-specific values (commands, generated paths, naming pattern, trunk) come from
[`.claude/project.env`](../project.env), filled by `/adapt-to-project`. An empty value turns the
check off, so every hook no-ops harmlessly until the project is adapted.

| Hook | Fires on | What it does |
| :--- | :------- | :----------- |
| `quality-checks.sh` | `Stop` | When source files (`SOURCE_EXTENSIONS`) are dirty: `FORMAT_FIX_CMD` then `LINT_CMD` on the dirty files of the session's tree (its cwd when that is a worktree of this repository, else the project checkout). Blocks (exit 2) on a lint failure, showing the last 40 lines. Typecheck and tests are deliberately not run here: `scripts/verify.sh` runs them before a push. |
| `convention-spot-check.sh` | `Stop` | Runs the checks in [`.claude/spot-checks.tsv`](../spot-checks.tsv) (path regex, forbidden content regex, message) over the session's changed and untracked files. Blocks once (exit 2) so findings actually reach the model (exit-0 output never does), then stays silent on the `stop_hook_active` re-run so a heuristic misfire cannot loop. Comment quality is `comment-pruner.sh`'s. |
| `comment-pruner.sh` | `Stop` | When the session added net-new comments (hashed against a memo of already-adjudicated ones): exits 2 so the main loop dispatches the `comment-pruner` subagent over the touched files, then seals the memo. A cheap pre-filter — a response that adds no comment pays nothing. |
| `git-safety.sh` | `PreToolUse(Bash)` | Blocks `rm -rf`, `DROP TABLE`, `git push --force`, `git reset --hard`, `checkout -b` on the trunk, and pushes to the trunk. Trunk name from `GIT_TRUNK` (default `main`). |
| `push-gate.sh` | `PreToolUse(Bash)` | Asks the user before a push that can create a branch on `origin`, unless the branch is listed in `$AGENT_APPROVED_PLANS`, and before a `gh` call that can merge. Only a plain `git push origin <branch>`, alone in its call, can skip the prompt. Registered with `onFailure: block`, so a crash does not let a push through. |
| `frozen-paths.sh` | `PreToolUse(Edit\|Write\|NotebookEdit)` | In a session that sets `$AGENT_FROZEN_ALLOW`, blocks an edit to a path matching `FROZEN_PATHS_REGEX` unless that file holds a `<branch> <path>` line for it. Off when either is unset. |
| `tool-result-size.sh` | `PostToolUse` | Warns the model when one tool result is large enough to crowd its context (10k characters for a verify log, 20k otherwise), with a hint for the next call. Never cuts the result. |
| `protect-generated.sh` | `PreToolUse(Edit\|Write)` | Blocks edits to paths matching `GENERATED_PATHS_REGEX` (regenerate from the source instead). |
| `validate-file-naming.sh` | `PreToolUse(Write)` | Blocks new files under `FILE_NAMING_SCOPE_REGEX` whose name doesn't match `FILE_NAMING_REGEX`, showing `FILE_NAMING_HINT`. Overwrites of existing files and paths outside `$CLAUDE_PROJECT_DIR` pass, so legacy names and sibling repos are not held to the pattern. |
| `post-compact-state.sh` | `SessionStart(compact)` | After compaction, re-injects live state read fresh: the session's directory, the project checkout's branch, the other worktrees (path and branch), the open PRs and whether the CI review covers their head. It never runs `git status`, which would run commands another session can plant in git config. |

**Configure / disable:** edit the entry under `hooks` in `settings.json`. Scripts must stay
executable (`chmod +x`). Each hook's hand-run cases live in `tests/hooks/`:
`bash tests/hooks/<name>.test.sh`. To add a hook, see [`.claude/README.md`](../README.md) ("New hook").

**Rust port, in progress:** the hooks are moving into one binary, `claude-hooks <hook-name>`, built
from [`tools/hooks-rs`](../../tools/hooks-rs). It holds `git-safety`, `frozen-paths`,
`tool-result-size`, `protect-generated`, `validate-file-naming`, `quality-checks`,
`convention-spot-check`, `comment-pruner`, `post-compact-state` and `push-gate` so far, plus most of
`scripts/`: `pre-commit`, `pre-push`, `added-lines`, `check-dashes`, `worktree-create`,
`worktree-remove`, `gate-lock`, `check-agent-config` and `verify`. `settings.json` and the git hooks still call the shell files until every one is
ported. Its cases are `cargo test --release` there, and
the `Hooks` workflow runs them with clippy on each PR that touches the crate.

Two subcommands have no shell twin and nothing calls them yet; the worker launcher that wires them
comes in a later change:

| Subcommand | Event | What it does |
| --- | --- | --- |
| `worker-permission` | `PermissionRequest` | Decides for a session started with `AGENT_WORKER=<handle>-<issue number>`; any other session gets no decision. Allows two kinds of sandboxed Bash call. A `gh issue` call that names the launcher's repository with `-R` or `--repo` (equal to `AGENT_REPO`; unset means no such allow): a read of any issue, or a comment or new body on the worker's own issue, given inline with `--body` (never a file). A lone `git push origin <prefix>/issue-<N>` of the checked-out branch of the worker's own issue, to the URL the file `AGENT_APPROVED_PLANS` names lists for it. Refuses a push in any other shape. Every other call is recorded under `requests/` in the state directory (`AGENT_WORK`, else `~/.agent-work/<main checkout name>`), announced in `lead-inbox.txt`, and denied with its request id until a grant for that exact call exists. A grant is used once, lasts 30 minutes, and covers the call only from the same directory and, for Bash, at the same HEAD with every tracked and untracked file unchanged (ignored files are not watched). |
| `grant show <id>`, `grant allow <id>` | run by the lead | `show` prints the recorded call with every character outside printable ASCII escaped. `allow` logs the call to `grants.log`, then writes the grant. Run `allow` only on the user's word for that call. |

The Rust `push-gate` asks in four cases `push-gate.sh` lets through:

- `<branch>` is not the local branch `refs/heads/<branch>`: a symbolic ref, a tag or `heads/<name>`, which git pushes to another ref.
- A `gh api` call writes a ref or a file (`git/refs`, `contents`, `update-branch`, a branch `rename`, or a GraphQL ref mutation). A read of the same paths passes.
- `push.followTags`, `push.recurseSubmodules` (other than `check`) or `submodule.recurse` is on, in any config file git reads: the push then carries tags or submodule commits no plan covers.
- Origin's push URL is not one `PUSH_GATE_REMOTES` lists. The gate looks a branch up on origin from outside the sandbox, at a URL the agent can set, so it reaches only the URLs that variable names in the `env` of `settings.json`. Entries are separated by spaces; each is a whole URL, a prefix ending in `/` (`https://github.com/`), or an scp-form prefix ending in `:` (`git@github.com:`; a `:` prefix that holds `://` matches nothing), followed only by letters, digits, `.`, `_`, `/` and `-`, spelt as `git remote get-url --push origin` prints it. Without the variable, every push of a branch the plans do not list asks.
