# Tooling audit

`/tooling audit`: find every place where our agent tooling (skills, agents, hooks, settings) disagrees with the trunk or with itself, and rank the fixes. A **finding** is a quoted claim plus the `path:line` that contradicts it; a claim with no contradicting line is not a finding.

The audit reads only this repo and your own settings files, and it edits nothing: its output is the ranked rows and the tickets the user picks.

## 1. Inputs

- The trunk's `.claude/`, `docs/`, `scripts/`, `AGENTS.md` and `CLAUDE.md` (trunk from `GIT_TRUNK` in `.claude/project.env`). Audit a checkout that sits at the trunk's head and note its sha: every `path:line` you cite is in this tree at that sha.
- `~/.claude/settings.json` and the main checkout's `.claude/settings.local.json`. Never quote a value under `env`, `apiKeyHelper` or `mcpServers`. A missing file means the source is absent.

Completion: you hold the sha and know which of the two settings files exist.

## 2. Fan out

Spawn 3 `general-purpose` agents in one message, each read-only (Read, Glob, Grep). Give each its area below, the absolute paths from step 1, and this return shape, one row per finding:

`area | claim (quoted) | where (path:line) | evidence (path:line) | fix (one line) | impact (high, medium, low) | owner (repo or user)`

`impact` is high when the line makes an agent act wrongly (a contradiction a worker follows, a permission gap that blocks a prescribed step), medium when it costs a turn or a token budget, low otherwise. `owner` is `user` for every row whose fix touches a settings file (`.claude/settings*.json`, `~/.claude/settings.json`, a permission rule included), since only the user writes settings; otherwise `repo`.

1. **Skills against the workflow.** Every line under `.claude/skills/` that states a rule `agent-workflow` or `AGENTS.md` also states: a contradiction, a stale `§` number, and any file, flag, script or command it names that does not exist in the tree.
2. **Agents and hooks.** `.claude/agents/*.md` against the skills that spawn them, and `.claude/hooks/*` against `.claude/README.md` and `.claude/settings.json`: a hook a setting names that does not exist, a hook no setting wires, a README row the hook code does not do.
3. **Settings and permissions.** `.claude/settings.json` and both settings files from step 1 against the commands the skills and `setup-agent-env/lead-tools` prescribe: a prescribed command no allow rule covers (it prompts, or refuses in a worker), a rule no prescribed command uses, a deny a skill steps on. Each row's fix is the exact JSON lines to add or remove, and the file.

Completion: each agent returns its rows, or "none" with the count of files it read.

## 3. Verify and rank

Read every row's `where` and `evidence` lines yourself and drop each row whose lines do not say what it claims. Merge rows that share one fix. Number the rest `T1`, `T2`, ... ranked by impact, then by area order. Completion: every kept row's two lines were read by you, in this step.

## 4. Settings diff

For each `owner: user` row that changes a settings file, note the file's path and only the JSON lines to add or remove, each line starting `+ ` or `- `. The user pastes them by hand (`agent-workflow` §8), so write no settings file.

Completion: every `owner: user` settings row has its file and its `+`/`-` lines.

## 5. Hand over

Render the kept rows as `html-artifact`'s pick sheet (choices `ticket` and `skip`, the settings diff under the rows, the sha in the header), give the user the sheet and wait for the copied picks. File each `ticket` row through `file-ticket`, which decides between one ticket per row and one per shared fix. Completion: every `ticket` row has its ticket id.
