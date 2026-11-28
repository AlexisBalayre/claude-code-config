# Daily mode

`/notes daily plan|summary|status|checkin`: the user's daily logbook note, `<vault>/$OBSIDIAN_DAILY_DIR/YYYY-MM-DD.md`, synced with the tracker and git. The vault rules are in [SKILL.md](SKILL.md).

- **Template:** `<vault>/Templates/Daily Log.md`: `## Daily Plan` (Focus, Objectives, Tasks, Blockers), then `## Daily Summary` with three placeholder lines, **Output**, **Delta** and **Cache**. Copy those three byte-exact.
- **Tracker:** the assignee and team from the `Tracker metadata` section of `.claude/skills/file-ticket/SKILL.md`. A tracker task line is `- [ ] {title} ([{ID}]({url}))`, sorted by priority, Urgent and High first.
- **Git author:** `--author="$(git config user.name)"`.
- **Time:** read `date` in the same call that writes a stamp; local time.
- **A long past note:** find its headers with `grep -n '^#'`, then Read only the section you need.

## plan

Create today's note with its goals and the carryover.

1. Get today's date. When today's note exists, ask before overwriting it.
2. Read yesterday's note (the newest earlier file) and take every unchecked item of its **Cache**.
3. Fetch the user's `In Progress` and `To Do` tracker issues.
4. Fill the template:
   - open `## Daily Plan` with a `> Verified HH:MM local.` blockquote: the trunk's sha (`git ls-remote origin refs/heads/<trunk>`, the trunk from `GIT_TRUNK` in `.claude/project.env`), the open PRs, and any release or deploy state that gates the day. Check each sha and PR state yourself; a handoff's numbers go stale within hours.
   - **Focus:** the highest-priority issue. **Objectives:** the top 3.
   - **Tasks:** `In Progress on the tracker:`, then `To Do on the tracker:`, then a carried-from-yesterday group for Cache items not on the board.
5. Write `<vault>/$OBSIDIAN_DAILY_DIR/YYYY-MM-DD.md` and confirm with the task counts.

## summary

Update today's note with the day's progress.

1. Read today's note; stop when it is missing.
2. Fetch the day's activity:
   - git: `git log --oneline --since="today 00:00" --author="$(git config user.name)"`;
   - GitHub: merged today, `gh pr list --author=@me --state merged --search="merged:>=YYYY-MM-DD"`; still open, `gh pr list --author=@me`;
   - tracker: the issues moved to `Done` today.
3. Update the note:
   - **Tasks:** mark the completed items `[x]`.
   - **Output:** the merged PRs, one bullet each with what it did, and the commit shas.
   - **Delta:** what changed against the plan, as a short narrative.
   - **Cache:** every remaining `[ ]` task and follow-up, for tomorrow's `plan`.
4. Write the note and confirm the update.

## status

Terminal only, no write. Read today's note when it exists and fetch the current git and tracker data, then print the task ratio (`3/5 completed`), today's commits, and the tracker issues completed and in progress.

## checkin

A plain-text async update for a team chat. Fetch the `In Progress` tracker issues and read **Focus** and **Blockers** from today's note, then print exactly this, with no markdown (the name from `git config user.name`):

```text
{User Name}
Focus: {Issue Title 1} + {Issue Title 2} (concise and punchy)
Blocker: {from the note, or "None"}
Signal: {notable news, or "None"}
Links: {issue URLs, or "None"}
```
