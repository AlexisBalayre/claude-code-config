# Command catalog

Slash commands you invoke explicitly. They carry side effects (posting PR comments, writing
files), so they never auto-trigger.

This repo currently ships no standalone commands: the worked example that lived here
(`/code-review`, a multi-agent PR review) graduated into the
[`pr-ci-review` skill](../skills/pr-ci-review/SKILL.md) — same explicit `/pr-ci-review`
invocation (it sets `disable-model-invocation: true`), but with the reviewers factored out
into reusable [`review-*` subagents](../agents/README.md) and a CI-friendly structured output.

The layer itself is still useful: a command is a plain markdown prompt with frontmatter
(`description`, optional `allowed-tools`, `$ARGUMENTS` substitution) — the lightest way to
package a repeatable, side-effect-bearing workflow tied to your own tools (issue trackers,
static analysis, note-taking). Note that a manual-only skill covers the same ground with
better structure; reach for a command when a single self-contained prompt file is all you need.

To add a command, see [`.claude/README.md`](../README.md) ("New command").
