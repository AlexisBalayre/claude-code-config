# Skill catalog

Each skill is a `<name>/SKILL.md`. Claude sees only the one-line description at session start
and loads the full skill when it is relevant. This catalog says **when** each fires and **how**
to invoke it.

This template ships only the skills bound to its own machinery: the project profile, the hooks, the review pipeline and the agent docs. General engineering skills come from a plugin, see [Companion plugin](#companion-plugin).

**Invoke legend**
- **Auto or `/name`**: Claude triggers on the cue described; you can also run `/name` yourself.
- **Manual only**: you invoke it; Claude never auto-triggers (`disable-model-invocation: true`).

## Setup: fit the template to a project

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `adapt-to-project` | Right after copying the template into a repo, or when the project grows a new area. Surveys the codebase, fills `.claude/project.env`, `AGENTS.md`, the conventions docs and their rule loaders, the architecture/security/glossary docs, then prunes skills and agents the project can't use. Re-runnable. | Manual only (`/adapt-to-project`) |

## PR & review: from branch to merged

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `pr-description` | Draft or rewrite a PR title and body (Goal, What changed, Done with its proof, Merge danger), then create or update the draft PR via `gh`. Other skills call it when they open PRs. | Auto or `/pr-description` |
| `pr-ci-review` | Cost-optimal multi-agent code review of local changes or a PR: relevance-gated `review-*` subagents, a validation pass, and inline posting. | Manual only (`/pr-ci-review`) |
| `address-review-comments` | Triage, decide, challenge, and implement a PR's open review threads end to end, replying as you go, with a human in the loop. | Auto or `/address-review-comments` |
| `review-retro` | Mine past automated-review runs for recurring failures and propose fixes to the review setup as one PR. Needs CI-produced run history. | Manual only (`/review-retro`) |

## Agent config

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `writing-for-agents` | Create or edit a skill, an agent, `AGENTS.md`, or `CLAUDE.md` well: context pointers, information hierarchy, leading words, invocation mechanics, and this repo's house rules. | Auto or `/writing-for-agents` |
| `ci-and-agent-config` | Editing `.github/workflows/`, `.claude/hooks/`, `.claude/settings.json` or a shell script: the parser harness cases, the macOS against ubuntu gotchas and the live-run rule. | Auto or `/ci-and-agent-config` |

## Companion plugin

Planning, TDD, debugging, grilling, domain modeling, research, prototyping and handoff skills are not vendored here. Install [mattpocock/skills](https://github.com/mattpocock/skills), which ships them as a self-updating plugin:

```bash
claude plugin install mattpocock-skills@claude-plugins-official
```

Two things differ from the plugin's defaults in a repo built from this template:

- **Domain language.** The plugin's skills look for a `CONTEXT.md`. Here the vocabulary lives in `docs/glossary.md` and decisions in `docs/adr/`; tell the plugin's setup skill to use those paths.
- **Tracker.** This template's skills read the `TRACKER_*` keys in `.env`. The plugin keeps its own tracker setting; point both at the same project.
