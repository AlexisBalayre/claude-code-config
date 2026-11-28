# Wiz fix

`/wiz fix [--cve|--sast]`: burn down a **worklist** of open Wiz findings. Each finding passes a confidence **gate**: high-confidence and mechanical gets fixed automatically; everything else **escalates** to a human decision. All fixes land in one PR reviewed at merge. Wiz stays read-only: never mark a finding resolved or rejected there, a landed fix clears itself on the next rescan.

Scope from the arguments: default both sources; `--cve` or `--sast` narrows.

The router pre-approves only the Wiz tools, Read, Grep, Glob and `jq`, since this mode reads Wiz finding text. Every other call here (Edit, Agent, `git`, `gh`, the install and build commands) goes through the session's normal permission rules: in a worker, `agent-workflow` §1 Grants.

## 1. Build the worklist

Query through [AUDIT.md](AUDIT.md)'s loop, with [SKILL.md](SKILL.md)'s project resource IDs. Pull OPEN findings:

- **Container CVEs** (`vulnerabilities`): `list_vulnerability_findings` over the `container_repository` UUIDs from `${WIZ_CONTAINER_REPO_IDS}`, `severity: ["CRITICAL","HIGH"]`, `status: ["OPEN"]`, `has_fix: true`. Record per finding: image, CVE, `detailedName`, `version` to `fixedVersion`, `layerMetadata.isBaseLayer`, `artifactType.group`, `transitivity`, `hasCisaKevExploit` / `hasExploit`.
- **SAST** (`codesec`): `list_sast_findings` over the repo `resource_id` (`${WIZ_REPO_BRANCH_ID}`), `is_default_branch: true`, `status: ["OPEN"]`. Record: rule, severity, `filePath`:`startLine`, weakness (CWE).

Parse the persisted `tool-results/*.txt` with jq; dedupe. Print the worklist grouped by source and severity. Completion: every open finding is on the list with its fields, or the list is empty (stop and say so).

## 2. Branch

File one Chore ticket for the run through `file-ticket` first; whether it takes a Design is the Chore row of `.claude/skills/file-ticket/templates.md` (Type and size). A worker works in the worktree it was launched in (`agent-workflow` §2). A solo session runs `scripts/worktree-create.sh wiz-fix-<yyyy-mm-dd>` and works in `.worktrees/wiz-fix-<date>` via `git -C` (never the trunk, `GIT_TRUNK` in `.claude/project.env`).

## 3. Triage each finding through the gate

Assign every finding exactly one verdict. **AUTO-FIX** only when the fix is mechanical, semver-safe, and verifiable AND the weakness is confirmed real. Otherwise **ESCALATE**. Never contort code to silence a false positive; tag it **FP** with a reason instead.

**Container CVE:**
- AUTO-FIX: `fixedVersion` exists and the bump stays within the same major (patch or minor), it is a dependency bump or a base-image patch bump, and verify (step 5) stays green. Order by `hasCisaKevExploit`, then `hasExploit`, then severity.
- ESCALATE: major-version jump, no fixed version, a direct pinned production dependency with breaking risk, or verify goes red.

**SAST** (default ESCALATE, the false-positive base rate is high):
- AUTO-FIX only if reading `filePath`:`startLine` confirms the weakness is real, untrusted input actually reaches it, and the fix is behaviour-preserving.
- FP: record with rationale, do not edit. Known FP shapes are in SKILL.md's "Before treating a finding as real". Escalate anything touching crypto or auth even when it looks mechanical.

For a large SAST list, fan the per-finding code verification out to parallel `Agent` (`review-security`) calls; keep the verdicts, not the transcripts. Brief each to judge its finding on existing code, not a diff, and return real, or a false positive with a reason.

Completion: every finding tagged AUTO-FIX, ESCALATE, or FP with a one-line reason.

## 4. Escalate (human in the loop)

Present the ESCALATE findings in batches, never one at a time: in a worker, to the lead (`agent-workflow` WORKER.md §2 step 2); otherwise via `AskUserQuestion`. Each carries its finding, why it missed the gate, and the choices (apply a named fix / skip / mark FP). Fold approved fixes into the AUTO-FIX set. Do not touch an escalated finding without a decision.

## 5. Apply and verify

Apply the AUTO-FIX set, grouped by kind:
- **Dependency bump**: raise the dep, or pin a transitive one through the package manager's override / constraint mechanism, to `fixedVersion`, then regenerate the lockfile (`INSTALL_CMD` in `.claude/project.env`, or the package manager's update command).
- **Base image**: bump the `FROM` tag or digest in the Dockerfile.
- **SAST**: the confirmed mechanical edit.

Verify: commit each group separately, then run `bash scripts/verify.sh` on the clean tree (pre-push refuses a push without its stamp), plus the project's build after a dependency bump. Image base-bumps cannot build locally, so rely on the release image rescan and say so. A red verify sends the finding back to ESCALATE, never into the PR: revert its group's commit (`git revert`), commit the group's other fixes again without it, and verify again.

## 6. One PR

Push under the approved plan (in a worker, `agent-workflow` §1 and §4), then open a single PR (house style: call the Skill tool with "pr-description"). Its `## What changed` is one table, a row per Wiz finding with its status (fixed: the bump or edit; escalated or deferred: the decision; false positive: the rationale, for a human to mark in the Wiz portal). Link the Wiz findings. Reviewed at merge.
