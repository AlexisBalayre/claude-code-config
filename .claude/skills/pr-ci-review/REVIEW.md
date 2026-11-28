# Review rules

What a review round asks of the author and when a PR is ready to merge. `pr-ci-review` reads this file to carry findings forward; `address-review-comments` reads it to answer them. CI restores this directory from the base branch, so a PR cannot rewrite the rules it is reviewed under.

## 1. Rounds

- A round is one review of a head: the first is full, and a push after it gets an incremental review of the delta. `@claude review full` asks for a full one.
- Minors (`nit`, `pre-existing`) never block and need no reply.
- The author fixes each blocking `important` (§2), then pushes; the push starts the next round.
- A clean round ends the rounds: do not push again to draw another review.
- Merge the base into the branch only when GitHub reports the PR `CONFLICTING`, or in a push you were making anyway: a merge commit alone buys a paid round that reviews nothing of yours.

## 2. Importants block

- An `important` is fixed before merge, never deferred to a follow-up.
- An author who holds a finding wrong challenges it with code evidence, in its thread or in one PR comment: the file and line that show the claim does not hold. The next round reads that answer, but only a fix on the head resolves the finding in the record.
- The user decides a challenge the evidence does not settle. Their 👎 on the bot's comment is the record of a finding ruled wrong.

## 3. Merge

A PR is ready when the latest full review on its head has zero open importants, apart from one the user ruled wrong (§2). The `claude-review` commit status carries that verdict: a quick or area-restricted round is partial and leaves the status to a full pass. An open minor never counts.
