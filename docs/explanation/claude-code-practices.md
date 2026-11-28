# Claude Code practices

The practices this template is built on: what to do, why, and where the template applies it. Each one came from running Claude Code on a real codebase, often with several sessions in parallel. Limits and flag names belong to Claude Code and move between versions, so check them again after an upgrade.

A row with a path is enforced or wired by this template. A row without one is a habit to keep.

## Choose the setup

| Practice | Why | Where |
| --- | --- | --- |
| Default to one session. | Each parallel agent pays its own start-up context and its own coordination turns. Parallel work costs more tokens for the same tasks, so it must buy wall-clock time you need. | |
| Grow in steps: one session, then subagents, then two or three sessions, then a lead session. | Add a lead only when relaying between sessions is the bottleneck. | |
| Delegate reads to subagents, keep decisions and writes in the main session. | A subagent's file dumps stay out of the main context; only its conclusion comes back. A write needs one owner. | `.claude/agents/` |
| One worktree per editing agent, created by script. | Two agents in one checkout overwrite each other. The remove script keeps a worktree that still holds unpushed work. | `scripts/worktree-create.sh`, `scripts/worktree-remove.sh` |
| One agent per package or area at a time. | Sibling PRs in one area conflict and force repeated merges of the trunk. | |
| A lead session routes, asks and merges. It is not a quality gate. | A lead that followed the work is not independent of it. The automated review judges. | `docs/workflow.md` |

## Instructions and precedence

| Practice | Why | Where |
| --- | --- | --- |
| State a precedence ladder, with hooks as the mechanical layer under it. | Agents meet conflicting instructions daily. A fixed order ends the guessing, and a blocked command goes to the user as is. | `AGENTS.md` §Precedence |
| Keep `AGENTS.md` tool-neutral; `CLAUDE.md` imports it and adds Claude-only lines. | Claude Code reads `AGENTS.md` only when no `CLAUDE.md` exists. Imports resolve four hops deep at most. | `CLAUDE.md` |
| Core rule: read 2-3 similar files and match them. | Matching neighbours gives more consistency than a style guide, for the price of a few reads. | `AGENTS.md` §Role |
| Comments carry only a non-obvious WHY; audit every discretionary construct. | Agents over-comment and over-abstract by default. The audit makes each helper, option or defensive branch earn its place. | `docs/conventions/core.md` |
| Conventions are path-scoped rules, symlinked, never `@import`ed. | An `@import` in a rule file loads every doc into every session. A symlink loads one doc on the first touch of its area. | `.claude/rules/`, `tests/scripts/conventions-rules.test.sh` |
| Convention docs hold obligations only. | Tutorials and system description cost tokens on every load. No line tells the model to Read a doc: the rule already injected it. | `docs/conventions/` |
| Hard rules live in hooks, not in prose. | Prose rules are broken under load. A rule broken twice moves into a hook. | `.claude/hooks/` |
| Keep skills short and router-style. | Keep `SKILL.md` under 500 lines and move detail to sibling files. Descriptions are capped (1,536 characters) and load into every session; `disable-model-invocation: true` keeps a user-only skill's description out. | `.claude/skills/` |
| Write prompts as goal, constraint, check. | State the goal, the constraint and how to verify it; name the files; ask for the smallest diff. | `writing-for-agents` skill |

## Context

| Practice | Why | Where |
| --- | --- | --- |
| Measure the start-up floor with `/context`. | Tool definitions, skill descriptions and memory load before any work, and every later turn re-reads them. Measure again after each Claude Code upgrade. | |
| Launch unattended or worker sessions slim. | `--strict-mcp-config` with only the servers the task needs, `--disallowedTools` for unused tools, and skills the session never uses switched off. | |
| Put a worker's standing prompt in `--append-system-prompt-file`. | A system prompt survives compaction; a first chat message does not. | |
| Read files with the Read tool, not `cat` or `sed`. | Shell reads fire no path rule, so the area's conventions never load. MCP results fire none either. | `CLAUDE.md` |
| Warn on oversized tool results. | One large log stays in context for the rest of the session. The hook names a cheaper call. | `.claude/hooks/tool-result-size.sh` |
| Batch independent reads in one message. | Each turn re-reads the whole context: three parallel reads cost one re-read, three sequential reads cost three. | |
| Start each session with its cwd inside its worktree. | A session whose cwd sits outside its worktree can load the same rule twice. | |

## Tokens and usage

| Practice | Why | Where |
| --- | --- | --- |
| Know the cost shape: cache reads dominate. | Every turn re-reads the context from cache, so the price of a session follows context size times turn count far more than output. | |
| Keep calls small. | Cost grows faster than context size. A session held near its limit costs several times one kept lean. | |
| Don't change model, effort or MCP servers mid-session. | Each change breaks the prompt cache and the next turn pays a full rewrite. Set them at launch. | |
| Pick the model by task size. | Small, well-specified changes run fine on a smaller model at high effort. Keep the largest model for design and hard debugging. | |
| No acknowledgements between agents; batch questions. | Every message to an agent wakes it and re-reads its whole context. | |
| Turn prompt suggestions off in agent sessions. | They cost tokens on every turn and nobody reads them. | |
| Put a budget cap on every unattended call. | `claude -p` with `--max-budget-usd` bounds a job that loops. | |
| Move polling out of the model. | A shell loop or scheduler job watches CI for free; a model turn that polls re-reads the context each time. | |
| Watch the usage window before launching parallel work. | Many sessions drain a weekly limit in days. | |

## Compaction and memory

| Practice | Why | Where |
| --- | --- | --- |
| Cap the context early. | Compacting sooner keeps every call cheap. `CLAUDE_CODE_AUTO_COMPACT_WINDOW` sets the threshold for a session. | |
| Re-inject live state after compaction. | The summary can be stale. A `SessionStart` hook on `compact` prints cwd, branch, worktrees and open PRs. Output from a `PreCompact` hook does not reach the summary. | `.claude/hooks/post-compact-state.sh` |
| A new task gets a new session. | A long session compacts repeatedly, and each compaction costs a round of re-orientation. `/compact` is the tool inside a task. | |
| Keep state in handoff files with dated status lines. | Files survive what context does not. After a resume, read every sha and PR state again instead of trusting memory. | `handoff` skill |
| Memory ranks lowest and is re-verified. | Machine-local notes go stale. A rule every session needs belongs in the repo. | `AGENTS.md` §Precedence |

## Hooks and permissions

| Practice | Why | Where |
| --- | --- | --- |
| Give every hook a test harness. | A hook that fails open is worse than none, because everyone trusts it. | `tests/hooks/*.test.sh`, `scripts/check-agent-config.sh` |
| Know the hook semantics. | Exit 2 blocks. A `Stop` hook's exit-0 output never reaches the model, so an advisory `Stop` hook does nothing. Consecutive `Stop` blocks are capped, and each costs a turn. `"onFailure": "block"` makes a crash or timeout block. | `.claude/README.md` |
| Treat permission rules as text matches, not a boundary. | A deny rule matches the raw command text, and a quoted or reordered flag walks past it. Hooks parse the command and fail closed; the sandbox and token scopes are the real boundary. | `.claude/hooks/git-safety.sh` |
| Ask on merges, token reads and first pushes. | An allow rule broad enough to be useful also allows the irreversible call. | `.claude/hooks/push-gate.sh`, `permissions.ask` |
| Sandbox unattended sessions; refuse instead of prompting. | Nobody answers a prompt in an unattended session. | |
| A block goes to the user. | Rewording a command past a refusal defeats the guard. | `AGENTS.md` §Precedence |
| Guard the irreversible steps, measure the rest. | A deny list is never complete. Protect merge, delete, secrets and production; replay transcripts to see which blocks were noise. | |
| Settings belong to the user. | An agent never edits settings to unblock itself. Review `settings.local.json` now and then: a "don't ask again" given in one worktree applies in all of them. | `permissions.deny` |

## Review automation

| Practice | Why | Where |
| --- | --- | --- |
| Split the review by privilege. | Text in a PR can steer the model. The model job is read-only; separate jobs write. | `.github/workflows/claude-code-review.yml` |
| Area reviewers, then an adversarial validator. | Reviewers run only for the areas a diff touches, and the validator refutes findings before they are posted. | `.claude/agents/` |
| Minors never block. | A capped list of minors per round keeps the loop short. Each rides with another change, is declined, or is filed. | `pr-ci-review` skill |
| Don't pay for a review twice. | A push that leaves the reviewed diff unchanged keeps its verdict. | |
| Sweep the sibling sites. | A defect found at one call site often sits at the others. Check them before pushing the fix. | |
| Disable background tasks in CI. | The action keeps the first result; background subagents leave it hollow. Set `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1`. | |
| "Zero unresolved threads" proves nothing. | Findings without an anchor land in the review body. Read the latest review body. | |

## Verification gates

| Practice | Why | Where |
| --- | --- | --- |
| One verify command, a green stamp, a pre-push check. | An agent's "tests pass" is a claim. The stamp ties a green run to an exact tree. | `scripts/verify.sh`, `scripts/pre-push` |
| One heavy gate at a time across worktrees. | Parallel typechecks and test runs starve each other and fail in ways that look like flakes. | `scripts/gate-lock.sh` |
| A size cap on PRs. | Large diffs get weak reviews, from people and from models. | `scripts/added-lines.sh` |
| Name a proof before calling a PR ready. | A test that goes red without the fix, or a check run by hand. Checks that pass for the wrong reason are the common failure. | `pr-description` skill |
| Verify the merged tree. | Two green PRs can break the trunk together. Merge the trunk last, then verify. | |
| Never trust a piped exit code. | `cmd | tail` returns the status of `tail`. | |

## Shell gotchas

| Practice | Why |
| --- | --- |
| macOS has no `timeout`. | A script that calls it fails with exit 127. Use the script's own timer. |
| zsh is not bash. | No word-splitting of `$VAR`, arrays start at 1, a variable named `path` replaces `PATH`, and an unmatched glob is an error. |
| Keep scripts bash 3.2 and BSD compatible. | macOS ships bash 3.2, and its `sed` and `grep` lack GNU extensions such as `\b`. |
| `cd` does not carry reliably between calls. | Chain with `&&` or use `git -C`. A `;` chain keeps running in the wrong directory when the `cd` fails. |
| Quote heredocs. | An unquoted heredoc runs the backticks inside it. |
| Give every poll its own timeout. | A background poll with none runs until something kills it. |
