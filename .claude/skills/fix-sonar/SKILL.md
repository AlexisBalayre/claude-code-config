---
name: fix-sonar
disable-model-invocation: true
allowed-tools: mcp__sonarqube__search_sonar_issues_in_projects, mcp__sonarqube__change_sonar_issue_status, Bash(pnpm:*), Bash(git:*), Bash(gh:*), Agent
description: Orchestrate SonarQube issue resolution across the project using parallel agents and specific fix strategies.
---

# SonarQube Issue Fixer

This agent automates the identification and resolution of SonarQube issues. It prioritizes high-signal mechanical fixes and structured refactoring for complex rules.

> **Setup:** this skill talks to a SonarQube MCP server. Set `SONAR_PROJECT_KEY` in `.env`
> (see `.env.example`) to your project's key and enable the server in
> `.claude/settings.local.json`. The fix-strategy table below encodes rule-by-rule experience
> from this repo — adjust the false-positive rows to match your own findings.

## Operation Modes

### 1. Status Check (`--status`)

- **Action:** Retrieve all `OPEN` issues for project `${SONAR_PROJECT_KEY}`.
- **Grouping:** Aggregate by **Rule ID** and **Codebase Area** (API, Frontend, Engine, Gateway, Packages).
- **Display:** Present a summary table showing severity, count, and affected files.

### 2. Parallel Resolution (`--parallel`)

- **Batching:** Group issues by Rule + Area. Create up to 3 independent batches targeting different files.
- **Isolation:** Dispatch 3 parallel agents with `isolation: "worktree"`.
- **Workflow:** Fix -> `pnpm lint:fix` -> `pnpm typecheck` -> `pnpm test` -> Commit -> Push -> PR.
- **Constraint:** Never mix rules or areas within a single batch to prevent merge conflicts and context bleeding.

### 3. Targeted Fix (`<rule>` or `<rule> <area>`)

- **Action:** Resolve all instances of a specific rule, optionally restricted to a sub-directory/area.

## Fix Strategy & Batching Reference

| Rule      | Category   | Strategy                                                | Batch Size |
| :-------- | :--------- | :------------------------------------------------------ | :--------- |
| **S3776** | Complexity | Extract helpers, early returns, flatten nesting.        | 1-2 files  |
| **S107**  | Refactor   | Group parameters into a single options object.          | 1 file     |
| **S7763** | Export     | Systematic false positive here (locally-used barrel re-exports): mark `falsepositive` via `change_sonar_issue_status`, do NOT code-fix. | n/a |
| **S6819** | React/ARIA | Systematic false positive here (custom-viz ARIA roles): mark `falsepositive`, do NOT code-fix. | n/a |
| **S7781** | Modern JS  | Replace `.replace(/x/g, y)` with `.replaceAll('x', y)`. | 5 files    |
| **S6606** | Nullish    | Use `??` or `??=` instead of `\|\|` or manual checks.   | 3-5 files  |
| **S3735** | Promises   | Remove `void`, handle promises with `.catch()`.         | 3-5 files  |
| **S6759** | React      | Ensure props use the `Readonly<Props>` wrapper.         | 3-5 files  |
| **S2933** | TypeScript | Add `readonly` modifier to class properties/members.    | 5 files    |

## Agent Dispatch Template

Each parallel agent is initialized with the following instruction set:

> "Fix [COUNT] instances of SonarQube rule [RULE] in [AREA].
> **Context:** Refer to [FILE:LINE] list.
> **Verification:** Run `pnpm lint:fix && pnpm typecheck && pnpm test`.
> **Commit:** `fix: resolve SonarQube [RULE] [DESCRIPTION] in [AREA]`"

## Exclusions & Constraints

- **Auto-generated Files:** Skip `*.gen.ts`, `packages/acme-rpc/src/protos/generated/`, and any files in `dist/` folders.
- **Rule Mixing:** Under no circumstances should one PR contain fixes for multiple SonarQube rules.
