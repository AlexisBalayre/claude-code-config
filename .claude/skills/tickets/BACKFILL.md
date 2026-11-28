# Backfill mode

File Done tickets for your merged PRs that no ticket tracks, grouped one ticket per piece of work.

## 1. Date range

Take `--since YYYY-MM-DD` from the input. Without it, read the current cycle with `list_cycles` (`teamId`: the Team of `file-ticket`'s Tracker metadata, `type: "current"`) and use its `startDate`. Tell the user the date you use.

Done when the date is stated.

## 2. Merged PRs

```bash
gh pr list --author=@me --state=merged --search="merged:>=YYYY-MM-DD" --json number,title,url,mergedAt,body --limit 100
```

Stop if it finds none. Done when you hold the merged PRs since that date.

## 3. Drop the tracked ones

List the Assignee's tickets on the Team (`list_issues`, both from `file-ticket`'s Tracker metadata) and drop each PR whose URL is in a ticket's description or attachments, or whose body names a ticket (`Closes <KEY>-NNN`, `Refs <KEY>-NNN`; list those for the user). Report how many you dropped, and stop if none remain. Done when every PR left is one no ticket tracks.

## 4. Group

Group the rest by `file-ticket` §3's rule, one ticket per piece of work: the same conventional-commit scope, or related features and fixes. Map each group's type from its commits: `feat:` to Feature, `fix:` to Bug, `chore:`, `refactor:` and `test:` to Chore, `perf:` to Improvement, `docs:` to Documentation. Its points are your estimate on the scale of `templates.md` Type and size.

Done when every remaining PR sits in one group with a type and points.

## 5. Review

Show the groups:

```text
Proposed tickets (X groups from Y PRs)
Group 1: "<title>" [<type>] (N pts)
  - PR NNNN <title>
Skipped: Z PRs
```

The user may approve all, merge or split groups, rename, retype or re-estimate, or drop PRs or groups. Show the proposal again after each change. Done when the user approved it.

## 6. File

Write each approved group through `file-ticket` §2 and §5, with:

- **State** `Done` and the current cycle's id (the PRs are merged), read back as §5 says;
- **Body**: the shared core of `templates.md`, its Done-when items ticked (`- [x]`), plus `## PRs`, one line per PR as "PR NNNN <title>" with its URL;
- **Links**: each PR URL as a link attachment, so the next run drops these PRs at step 3.

Done when every group was read back, and the user has the digest: one line per ticket with its type, points and PR count, and the total points.
