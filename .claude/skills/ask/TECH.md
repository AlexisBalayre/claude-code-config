# Tech mode

`/ask tech <name or question>`: dig into a library, service, tool or technique we do not use yet, and end on a verdict the user can act on.

## 1. Frame the question

Restate the input as one question about our stack: "should `<tech>` replace or add to `<what we use>` for `<job>`?". Find the job in the code: grep `docs/glossary.md`, `docs/reference/architecture.md` and the dependency manifests for what does that job today, and `docs/adr/` for a decision that already settled it.

Done when the question names the tech, the job and what does it today (or that nothing does), and any ADR that touches it.

## 2. Sources

Read the tech's own docs, its changelog or release notes, its license and its issue tracker, with `WebSearch` and `WebFetch` (a fetch asks once per domain). Note for each claim the URL and the date or version it holds for. Prefer the primary source; a blog post or a benchmark counts only for what it measured, named as such.

Done when maturity (version, release cadence, maintainers), license, cost and the 2 or 3 claims that decide the verdict each have a cited source.

## 3. Fit with our code

Name where it would land (a package, a module, a seam), what it replaces or wraps, the `docs/conventions/` and ADRs it meets, and what a move costs: the call sites to change (`git grep` counts them), a migration, a new secret or a new service in the deployment.

Done when each fit claim carries a `path:line`, a count or an ADR.

## 4. Verdict

Open with one word, **adopt**, **try** or **skip**, then two sentences on why, then the cited claims from steps 2 and 3:

- **adopt**: it fits, and it beats what we have on a claim that matters to us;
- **try**: it looks right, but one open question decides it; name that question and the smallest run that answers it;
- **skip**: it does not fit, or loses to what we have; say what would change that.

Done when the verdict, its why and every claim behind it are written, with a source for each.

## 5. Offer the next step

Offer these and run only what the user picks. `/notes` and `/tickets` are user-invoked, so no skill can start them:

- **`/notes study <tech>`**: keep the dig as a cited study note. Hand the user that command, filled in, to type.
- **`/tickets file`**: an Epic that adopts it or runs the trial, its Goal and Why taken from the verdict but naming modules, not the verdict's `path:line` claims (`tickets/SKILL.md`'s Modules rule). Run `file-ticket` on it directly, the primitive `/tickets file` runs.
- **Both.**
- **A throwaway prototype**, when the verdict is `try` and running code answers its open question. Build it in the session's scratchpad (`$TMPDIR`), never in the repo, and keep it to: one command to run; state in memory, with a scratch file named "PROTOTYPE, wipe me" only when persistence is the question; no tests and no polish, only what makes it runnable; the full state printed after every action. Report what the run showed against the open question, and fold that into the verdict.

Done when the user has picked, and the picked step has run (or its command is handed over), or the user declined them all.
