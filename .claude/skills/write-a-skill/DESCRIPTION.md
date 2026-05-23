# Description Requirements

The `description:` field is **the only thing the agent sees** when deciding which skill to load. It's surfaced in the system prompt alongside every other installed skill. The agent reads these descriptions and picks the relevant skill based on the user's request.

## Goal

Give the agent just enough info to know:

1. What capability this skill provides.
2. When/why to trigger it (specific keywords, contexts, file types, observable user intent).

## Format

- Max 1024 chars.
- Write in third person.
- First sentence: what it does.
- Second sentence: "Use when [specific triggers]".

## Good example

```
Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDF files or when user mentions PDFs, forms, or document extraction.
```

The triggers are concrete: file types ("PDF files") and keywords ("PDFs, forms, or document extraction"). An agent can pattern-match these against incoming user requests.

## Bad example

```
Helps with documents.
```

Nothing for the agent to distinguish this skill from a half-dozen others. No file types, no keywords, no observable intent. Will never trigger correctly.

## Common failure modes

- **Generic verbs** ("helps with", "handles", "manages") that match too many requests.
- **No trigger phrase.** If the description doesn't include "Use when...", the agent has to guess. It will guess wrong.
- **Triggers that overlap with another skill.** If two skills both trigger on "create a route", the agent will pick the wrong one half the time. Make the triggers narrower (e.g. "create a frontend route" vs "create an API route") or disambiguate in the description body.
- **Marketing copy** ("powerful", "comprehensive", "production-grade"). The agent doesn't care; it costs tokens and crowds out the actual trigger surface.
