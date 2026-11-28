---
name: address-review-comments
description: Works a PR's open review comments end to end with the human deciding each call. Use when addressing a PR's review comments.
argument-hint: "[PR number (defaults to the current branch's PR)]"
---

# Address review comments

Take the open review comments on a PR from "what is the reviewer really asking" through to an implemented resolution. You steer; the human decides every judgment call. Handles every open thread, human or bot. Closed and resolved threads are context only.

## Process

### 1. Gather

Resolve the PR (the argument, or the current branch's PR via `gh pr view`). Pull its open review threads through `gh` / the GraphQL API: each comment with its file, line, author, body, and thread state. Resolved threads are context, not work. Always read the **actual current code**, not your recollection of it; the user may have changed it.

Done when every open thread is listed with its file, line, author and the current code it points at.

### 2. Triage

Sort each open thread into:

- **settled**: a pure mechanical fix, or the author already agreed an approach in-thread. Goes straight to implementation.
- **needs a decision**: everything else.

Done when every thread is settled or needs a decision.

### 3. Decide (no code yet)

For each *needs a decision* thread, investigate the reviewer's underlying concern, not just the symptom they pointed at, until you can lay out the realistic options with their trade-offs. Then route by what is actually in dispute:

- **Correctness dispute** ("this finding is wrong / a false positive"): spawn a validator subagent that adversarially re-checks the finding against the actual code, given the user's rebuttal. One round only. It returns **concede** (a false positive, with a code-grounded reason) or **defend** (the issue holds, with the concrete evidence). Bias to concede when it cannot defend with evidence: the user raised the doubt, so the burden is on the finding.
- **Design or approach decision**: call the Skill tool with "grilling" rather than settling it unilaterally; always provide your recommended answer. If the comment opens an architectural, ADR-worthy decision (hard to reverse, or it shifts a documented term or boundary), also call the Skill tool with "domain-modeling", so the resolution lands in the repo's vocabulary or an ADR.

No code is written in this phase. The goal is alignment. The validator and the grilling session inform the decision; **the user is the final authority** on every thread.

Done when each needs-a-decision thread has its options and a recommendation.

### 4. Checkpoint

If anything went through step 3, present a per-thread summary (the concern, the options considered, the decision reached, and the concrete change or an explicit "leave as-is" with rationale) and get the user's sign-off before implementing; in a worker, the summary goes to the user through the lead (`agent-workflow` §1 Approvals), and you implement on the relayed answer. When every thread was already settled, skip straight to implementation.

### 5. Implement

Apply the agreed changes. Each fix targets only what its comment raised: no opportunistic cleanup, no unrelated refactors, no removing ticket-referenced TODOs. Note any tangential improvements separately in the summary and ask before touching them.

Done when each agreed change is committed and nothing else changed.

### 6. Reply and resolve

In a worker session, push per `agent-workflow` §4 and post only the replies `agent-workflow` §6 lists; that section also says which threads you resolve and which stay open for the user.

In a solo or lead session, ask the user whether to commit/push. A bot thread gets no reply: the one post is REVIEW.md §2's code-evidence answer to an important the user challenges. A human reviewer's thread gets only the reply the user approves. After the push, set each thread's state:

- **Addressed by a change**, or **agreed, no change needed**: resolve the thread (GraphQL `resolveReviewThread`).
- **Conceded challenge** (the finding was a false positive): resolve; the user's 👎 on the bot's comment is the record (REVIEW.md §2).
- **Leave-as-is** (intentional, out-of-scope, or a human reviewer's point the user overrides): leave the thread **open**; a disagreement stays open for the user to close. A blocking `important` (`.claude/skills/pr-ci-review/REVIEW.md` §2) is never left as-is: fix it, or challenge it (step 3); it stays open and blocking until it is fixed or the user rules it wrong, and later rounds still list it.

## Invariants

- The user may change the code at any point; work from, and commit, the actual current state, not your recollection.
- Only change what each comment asks for.
- The human is the final authority on every thread; validators and grilling sessions inform, they do not decide.
- A conceded challenge and a leave-as-is pushback are both signals the review retro will later read. Handle them per step 6, honestly; do not scrub the disagreement record (the open thread, the user's 👎).
