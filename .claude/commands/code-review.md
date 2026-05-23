---
allowed-tools: Bash(gh pr comment:*), Bash(gh pr diff:*), Bash(gh pr view:*), Bash(gh pr list:*), mcp__github_inline_comment__create_inline_comment
description: Review the code of a pull request with high-signal filtering and context-aware validation.
---

# Pull Request Code Review Protocol

Provide a high-signal, context-aware code review for the specified pull request.

## Review Constraints

- **Assumptions:** All tools are functional; do not perform exploratory calls.
- **High Signal Only:** Only flag compilation/syntax errors, clear logic failures, unambiguous CLAUDE.md violations, and critical security/config misconfigurations.
- **Exclusions:** Do NOT flag style, nitpicks, pre-existing issues, or anything a linter/Biome would catch.

## Execution Steps

### 1. Identify Context (Haiku Agent)

Return a list of file paths for all relevant `CLAUDE.md` files:

- Root `CLAUDE.md`.
- `CLAUDE.md` files in directories (or parent directories) of files modified in the PR.

### 2. Summarize Changes (Sonnet Agent)

View the PR and return a concise summary of the intent and technical changes.

### 3. Parallel Review (6 Independent Agents)

Each agent receives the PR title, description, and diff.

- **Agents 1 + 2 (Sonnet):** Audit for `CLAUDE.md` compliance. Only apply rules scoped to the specific file path.
- **Agent 3 (Opus):** Scan for obvious, significant bugs in the diff alone. Ignore context outside the diff.
- **Agents 4 + 5 (Opus):** Identify security issues, logic errors, or incorrect implementations within the introduced code.
- **Agent 6 (Opus):** Contextual review. Read project docs, configs, and referenced source files. Check for spec contradictions, protocol violations (CORS, HTTP), and infra anti-patterns (unpinned images).

### 4. Validation (Parallel Subagents)

For every issue found in Step 3, launch a validator:

- **Bugs/Logic:** Use Opus to confirm the issue is real with high confidence.
- **CLAUDE.md:** Use Sonnet to verify the rule is scoped correctly and actually violated.
- **Context:** Verify the doc/spec actually claims what the reviewer stated.

### 5. Final Filtering

Filter out any issues not validated in Step 4. Exclude pedantic nitpicks, linter-catchable issues, and pre-existing drift.

### 6. Reporting

- **If no issues remain:** Post via `gh pr comment`:
  > ## Claude code review
  >
  > No issues found. Checked for bugs, CLAUDE.md compliance, and spec/config correctness.
- **If issues exist:**
  1.  Create a private list of unique comments to prevent duplicates.
  2.  Post inline comments via `mcp__github_inline_comment__create_inline_comment`.
  3.  **Format:** Brief description + cite/link to the specific `CLAUDE.md` rule.
  4.  **Suggestions:** Only provide committable blocks for small, self-contained fixes that resolve the issue entirely.
  5.  **Links:** Use the format: `<https://github.com/OWNER/REPO/blob/FULL_SHA/path/to/file#L[start]-L[end]>` with the full git SHA.

## Todo List

- [ ] Fetch PR details and diff.
- [ ] Locate relevant `CLAUDE.md` files.
- [ ] Distribute review tasks to parallel agents.
- [ ] Validate and filter findings.
- [ ] Post summary or inline comments as required.
