# Hunt mode

`/bugs hunt <area>`: point `pr-ci-review`'s reviewers at an area's code as it stands, validate what they raise, and file what survives. At most 5 agents and 5 new tickets a run.

## 1. Resolve the area

The input is a path, a glob, or a conventions area (a `docs/conventions/<area>.md` whose `paths:` frontmatter gives the globs). List its tracked source files at the checkout's HEAD with `git ls-files`, leaving out tests, fixtures, mocks, snapshots and generated files, and count their lines with `wc -l`. Name the HEAD sha (`git rev-parse HEAD`): every finding is anchored to it. A Bug's `## Cause` cites a trunk sha (`file-ticket/templates.md`), and the reviewers read the working tree, so the tree must be clean (`git status --porcelain` prints nothing) and HEAD must equal `refs/remotes/origin/<trunk>` (the trunk from `GIT_TRUNK` in `.claude/project.env`; the full name, which no local branch or tag can shadow); otherwise say which fails and stop. That ref is the trunk's tip as of the last fetch, so the digest says so beside the sha.

Done when the file list, its line count and the sha are named, on a clean tree at the trunk's tip. Over about 1,500 lines, propose a narrower area (a module, a route and its service) and ask.

## 2. Pick the roster

The rows of `pr-ci-review/SKILL.md` §4 decide who runs, read against the area instead of a diff:

- `review-correctness`, always; a second instance past about 800 lines, the two split by module, never overlapping;
- `review-security` and `review-perf`, each when its row gates it in, read against the area;
- then one `review-validator` (step 4).

One instance each otherwise: 5 agents at most, the validator counted.

Done when each reviewer is named with its slice of the files, and the total, validator included, is 5 or fewer.

## 3. Brief and spawn

Brief each reviewer in your own words with:

- **Its scope**: the file list (its slice of it) at the sha. Those files take the place of what `pr-ci-review` calls "the change": every line is in scope, so no finding is tagged `pre-existing`; tag each `important` or `nit`.
- **The run**: local, so the spike its manifest allows is fair when reading cannot settle a question, throwaway and trace-free.
- **The conventions it cites**: the `docs/conventions/*.md` whose `paths:` cover the files, read with `Read` before it cites one.
- **Known false positives**: the list `pr-ci-review` hands its reviewers, when the repo keeps one; a finding that makes an entry's claim is not raised unless the code shows it now holds.
- **The contract**: each finding as `{file, line, area, confidence (high or medium), tag, description}`, an `important` also with `body` (the concern, the code it cites, why it is reachable); and its final message is the deliverable.

Send every reviewer in one message, and wait for every report.

Done when every reviewer has reported. Dedupe across reports by root cause, keeping every site and the most severe tag, as `pr-ci-review/SKILL.md` §6 does. Then check each tag against its body, as §6 also does: an `important` whose body concedes no reachable break drops to `nit`, and doubt keeps it `important`. That set's size is the count **before validation**.

## 4. Validate

Spawn one `review-validator` with every finding, its `file:line` and the sha, the brief's scope, and the known false positives. Take its verdict on each: confirmed or refuted.

Done when every finding has a verdict. The confirmed set's size is the count **after validation**.

## 5. File

Rank the confirmed importants by reach and confidence. Run `file-ticket` on them in rank order until 5 tickets are filed; the cap counts tickets, not findings, since its §3 grouping can file several findings as one ticket. Each ticket is a Bug with a `## Cause` at `path:line` on the sha and the reviewer's evidence as its `## Repro`; its §2 search can make one a duplicate or a fold, which counts toward the 5 all the same. The importants left once 5 tickets are filed and every nit are filed only when the user picks them from the digest.

Done when each filed important (5 tickets at most) maps to a ticket id, new or existing, and the user has the digest:

- the area, its line count and the sha;
- the agents spawned, by name, and their count;
- the findings before and after validation, and the refuted ones with the validator's reason;
- the tickets filed, one line each;
- the unfiled importants, ranked, then the nits, each with its `path:line` and one line.
