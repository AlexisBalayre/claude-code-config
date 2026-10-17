import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { z } from "zod";
import { type Finding, reviewSummarySchema } from "./review-metrics.schemas";

/**
 * Recall eval for the CI review. Each case replays a merged PR with one known
 * regression planted in its own diff, or a PR left clean to count false
 * positives, and asks whether an important finding lands on the plant. Every
 * plant passes its PR's own tests and typecheck, so a hit is recall only the
 * reviewers supply.
 *
 * Without --run it plants each case and prints the planted hunk, which spends
 * nothing. With --run it pushes a `pr-<random>` branch and its `-base` per case,
 * opens a draft PR on them under the replayed PR's own title and body (the
 * workflow reviews a draft only on a `@claude review` comment), runs
 * /pr-ci-review against it locally the way the workflow invokes it, then closes
 * the PR and deletes the branches. Nothing in the PR, its branch or the folders
 * around its checkout names the eval, the case or the plant: the plant-named
 * index files and results.json stay in a `review-eval-*` folder of this repo's
 * git directory. A case with a `spec` opens it as an issue the PR closes, which
 * is what spawns the spec reviewer, and deletes the issue afterwards; its
 * requirements reach the reviewer through that issue alone. --pipeline <ref>
 * picks whose `.claude/` reviews (default HEAD), so two pipeline versions can
 * be scored on the same cases; --case <id> runs that case alone.
 */

const CASES_FILE = new URL("../eval/cases.json", import.meta.url);
const WORKFLOW_FILE = ".github/workflows/claude-code-review.yml";
// The workflow's config restore, narrowed to what this repo carries.
const CONFIG_PATHS = [".claude", ".mcp.json", "CLAUDE.md", "AGENTS.md"];
// The skill's own allowed-tools grant this locally, where no managed setting
// makes the lock the only rules; CI drops it. It would put the PR's config back
// over the base branch's.
const LOCAL_DENY = ["Bash(gh pr checkout*)"];
// A finding this close to the plant is about it: reviewers anchor on the line
// they reason from, which can sit a few lines off the edited one.
const LINE_WINDOW = 10;
const CASE_BUDGET_USD = 5;
const DEFAULT_BUDGET_USD = 10;
// Below this, a case would hit its cap before its reviewers return.
const MIN_CASE_BUDGET_USD = 1;

const caseSchema = z.object({
  id: z.string(),
  pr: z.number().int(),
  base: z.string(),
  head: z.string(),
  spec: z.string().optional(),
  plant: z
    .object({
      file: z.string(),
      find: z.string(),
      replace: z.string(),
      keywords: z.array(z.string()),
      regression: z.string(),
    })
    .nullable(),
});
type EvalCase = z.infer<typeof caseSchema>;
type Plant = NonNullable<EvalCase["plant"]>;

interface Outcome {
  id: string;
  verdict: "hit" | "miss" | "clean" | "false-positive" | "error";
  cost_usd: number | null;
  duration_ms: number | null;
  importants: string[];
  refuted_on_plant: string[];
  permission_denials: number;
  error: string | null;
}

function git(args: string[], options: { env?: NodeJS.ProcessEnv; input?: string } = {}): string {
  return execFileSync("git", args, { encoding: "utf8", ...options }).trim();
}

function gh(...args: string[]): string {
  return execFileSync("gh", args, { encoding: "utf8" }).trim();
}

/**
 * The managed permission lock in the workflow at `pipeline`, so a run grants
 * what that version of CI grants and no more.
 */
function lockedPermissions(pipeline: string): { allow: string[]; deny: string[] } {
  const workflow = git(["show", `${pipeline}:${WORKFLOW_FILE}`]);
  const lock = /managed-settings\.json[^\n]*<<'EOF'\n([\s\S]*?)\n\s*EOF\n/.exec(workflow)?.[1];
  if (!lock) throw new Error(`no managed-settings lock in ${WORKFLOW_FILE} at ${pipeline}`);
  return z
    .object({ permissions: z.object({ allow: z.array(z.string()), deny: z.array(z.string()) }) })
    .parse(JSON.parse(lock)).permissions;
}

/**
 * Rebuilds the case's head commit with the plant folded in, without a checkout:
 * the head's own message, parents, author and dates, minus the ` (#N)` a squash
 * merge appends, which would point the reviewer at the unplanted original; a
 * spec case squashes onto its base instead (below). A clean case goes through
 * the same rebuild, so it looks no different from its planted twin.
 */
function plantCommit(evalCase: EvalCase, scratch: string): { sha: string; line: number | null } {
  const { plant, head } = evalCase;
  let tree = git(["rev-parse", `${head}^{tree}`]);
  let line: number | null = null;
  if (plant) {
    // Raw, not git(): trimming would drop the file's final newline.
    const original = execFileSync("git", ["show", `${head}:${plant.file}`], { encoding: "utf8" });
    const at = original.indexOf(plant.find);
    if (at === -1 || original.indexOf(plant.find, at + 1) !== -1) {
      throw new Error(`${evalCase.id}: the plant's find string must occur exactly once in ${plant.file}`);
    }
    // A replacer function, so `$&` and the like in a plant are never expanded.
    const blob = git(["hash-object", "-w", "--stdin"], { input: original.replace(plant.find, () => plant.replace) });
    const mode = git(["ls-tree", head, "--", plant.file]).split(/\s/)[0];
    const index = { ...process.env, GIT_INDEX_FILE: join(scratch, `${evalCase.id}.index`) };
    git(["read-tree", head], { env: index });
    git(["update-index", "--cacheinfo", `${mode},${blob},${plant.file}`], { env: index });
    tree = git(["write-tree"], { env: index });
    line = original.slice(0, at).split("\n").length;
  }
  // A spec case's commit bodies restate its requirements, which must reach the
  // reviewer through the issue alone: its commits squash into one, under a bare
  // subject.
  const parents = evalCase.spec ? [evalCase.base] : git(["rev-parse", `${head}^@`]).split("\n");
  const message = evalCase.spec
    ? git(["log", "--no-merges", "-1", "--format=%s", `${evalCase.base}..${head}`])
    : git(["log", "-1", "--format=%B", head]);
  const [authorName, authorEmail, authorDate, committerName, committerEmail, committerDate] = git([
    "log",
    "-1",
    "--format=%an%n%ae%n%aI%n%cn%n%ce%n%cI",
    head,
  ]).split("\n");
  const sha = git(["commit-tree", tree, ...parents.flatMap((parent) => ["-p", parent])], {
    input: message.replace(/ \(#\d+\)$/m, ""),
    env: {
      ...process.env,
      GIT_AUTHOR_NAME: authorName,
      GIT_AUTHOR_EMAIL: authorEmail,
      GIT_AUTHOR_DATE: authorDate,
      GIT_COMMITTER_NAME: committerName,
      GIT_COMMITTER_EMAIL: committerEmail,
      GIT_COMMITTER_DATE: committerDate,
    },
  });
  return { sha, line };
}

function lineSpan(line: string | null): [number, number] | null {
  const match = /(\d+)(?:\s*[-–]\s*L?(\d+))?/.exec(line ?? "");
  return match ? [Number(match[1]), Number(match[2] ?? match[1])] : null;
}

function onPlant(
  finding: Pick<Finding, "file" | "line" | "description"> & { body?: string | null },
  plant: Plant,
  line: number,
): boolean {
  const span = lineSpan(finding.line);
  const near =
    finding.file === plant.file && span !== null && span[0] - LINE_WINDOW <= line && line <= span[1] + LINE_WINDOW;
  const text = `${finding.description}\n${finding.body ?? ""}`.toLowerCase();
  return near || plant.keywords.some((keyword) => text.includes(keyword.toLowerCase()));
}

/**
 * Checks the case out the way the workflow leaves the tree: PR head, base-branch
 * config, the PR's copy under .claude-pr/. The config's own permission grants
 * go, as CI's `allowManagedPermissionRulesOnly` discards them, so the rules
 * passed on the command line are the only ones left besides the skill's own
 * `allowed-tools` (see LOCAL_DENY).
 */
function checkout(branch: string, origin: string, pipeline: string, dir: string): void {
  // A clone of the PR branch alone, as CI checks out: a worktree would share this
  // repo's refs, `main` among them, where each plant's unplanted original sits.
  git(["clone", "-q", "--single-branch", "--branch", branch, origin, dir]);
  for (const path of CONFIG_PATHS) {
    if (!existsSync(join(dir, path))) continue;
    cpSync(join(dir, path), join(dir, ".claude-pr", path), { recursive: true });
    rmSync(join(dir, path), { recursive: true });
  }
  const present = CONFIG_PATHS.filter(
    (path) => spawnSync("git", ["cat-file", "-e", `${pipeline}:${path}`]).status === 0,
  );
  const archive = execFileSync("git", ["archive", pipeline, ...present]);
  execFileSync("tar", ["-x", "-C", dir], { input: archive });
  const settingsFile = join(dir, ".claude", "settings.json");
  if (existsSync(settingsFile)) {
    const { permissions: _repoGrants, ...settings } = JSON.parse(readFileSync(settingsFile, "utf8")) as Record<
      string,
      unknown
    >;
    writeFileSync(settingsFile, `${JSON.stringify(settings, null, 2)}\n`);
  }
}

function review(
  evalCase: EvalCase,
  prNumber: string,
  dir: string,
  repo: string,
  schema: string,
  lock: { allow: string[]; deny: string[] },
  budget: number,
) {
  const prompt = [
    "/pr-ci-review",
    "",
    `Pull request: #${prNumber} in ${repo}`,
    "Mode: full",
    `Merge base: ${evalCase.base}`,
    "Depth: standard",
    "Areas: gated",
  ].join("\n");
  const run = spawnSync(
    "claude",
    [
      "-p",
      prompt,
      "--model",
      "claude-opus-5-5",
      "--json-schema",
      schema,
      "--output-format",
      "json",
      "--permission-mode",
      "dontAsk",
      // The workflow loads no one's personal settings or MCP servers.
      "--setting-sources",
      "project",
      "--strict-mcp-config",
      "--no-session-persistence",
      "--max-budget-usd",
      budget.toFixed(2),
      // The workflow's own `--allowedTools "Glob,Grep"` on top of the lock.
      "--allowedTools",
      [...lock.allow, "Glob", "Grep"].join(","),
      "--disallowedTools",
      [...lock.deny, ...LOCAL_DENY].join(","),
    ],
    {
      cwd: dir,
      encoding: "utf8",
      maxBuffer: 256 * 1024 * 1024,
      env: {
        ...process.env,
        ANTHROPIC_DEFAULT_OPUS_MODEL: "claude-opus-5-5",
        ANTHROPIC_DEFAULT_SONNET_MODEL: "claude-sonnet-5-5",
        CLAUDE_CODE_DISABLE_BACKGROUND_TASKS: "1",
        // As in CI: credential-shaped variables never reach the shell the model
        // drives. gh still reads its own login.
        CLAUDE_CODE_SUBPROCESS_ENV_SCRUB: "1",
      },
    },
  );
  let result: Record<string, unknown> = {};
  try {
    result = JSON.parse(run.stdout) as Record<string, unknown>;
  } catch {
    // A crashed CLI prints no result; the exit status below reports it.
  }
  return {
    cost: typeof result.total_cost_usd === "number" ? result.total_cost_usd : null,
    duration: typeof result.duration_ms === "number" ? result.duration_ms : null,
    denials: Array.isArray(result.permission_denials) ? result.permission_denials.length : 0,
    summary: reviewSummarySchema.safeParse(result.structured_output),
    error:
      run.status === 0
        ? null
        : `claude exited ${run.status}: ${(run.stderr || String(result.result ?? "")).slice(0, 300)}`,
  };
}

function grade(evalCase: EvalCase, line: number | null, reviewed: ReturnType<typeof review>): Outcome {
  const base = {
    id: evalCase.id,
    cost_usd: reviewed.cost,
    duration_ms: reviewed.duration,
    permission_denials: reviewed.denials,
  };
  if (!reviewed.summary.success) {
    const error = reviewed.error ?? `structured output did not match the schema: ${reviewed.summary.error.message}`;
    return { ...base, verdict: "error", importants: [], refuted_on_plant: [], error };
  }
  const summary = reviewed.summary.data;
  const importants = summary.findings.filter((finding) => finding.tag === "important");
  const describe = (finding: { file: string | null; line: string | null; description: string }) =>
    `${finding.file ?? "?"}:${finding.line ?? "?"} ${finding.description}`;
  const { plant } = evalCase;
  if (!plant || line === null) {
    return {
      ...base,
      verdict: importants.length === 0 ? "clean" : "false-positive",
      importants: importants.map(describe),
      refuted_on_plant: [],
      error: null,
    };
  }
  return {
    ...base,
    verdict: importants.some((finding) => onPlant(finding, plant, line)) ? "hit" : "miss",
    importants: importants.map(describe),
    // Found and then refuted is a validator miss, not a reviewer one.
    refuted_on_plant: summary.refuted_findings.filter((finding) => onPlant(finding, plant, line)).map(describe),
    error: null,
  };
}

function main(): void {
  // The cases' paths are repo-relative, and ls-tree and update-index read paths
  // relative to the working directory.
  process.chdir(git(["rev-parse", "--show-toplevel"]));
  const args = process.argv.slice(2);
  const run = args.includes("--run");
  const pipelineAt = args.indexOf("--pipeline");
  const pipeline = git(["rev-parse", pipelineAt === -1 ? "HEAD" : args[pipelineAt + 1]]);
  const budgetAt = args.indexOf("--budget");
  let remaining = budgetAt === -1 ? DEFAULT_BUDGET_USD : Number(args[budgetAt + 1]);
  // NaN would pass every budget check below and cap no case.
  if (!Number.isFinite(remaining)) throw new Error(`--budget takes a USD amount, got ${args[budgetAt + 1]}`);
  const caseAt = args.indexOf("--case");
  const cases = z
    .array(caseSchema)
    .parse(JSON.parse(readFileSync(CASES_FILE, "utf8")))
    .filter((evalCase) => caseAt === -1 || evalCase.id === args[caseAt + 1]);
  if (cases.length === 0) throw new Error(`--case names no case in ${CASES_FILE.pathname}: ${args[caseAt + 1]}`);
  // Holds what names the plants (index files, results), so it sits where no
  // clone's surroundings lead: this repo's git directory.
  const scratch = mkdtempSync(join(git(["rev-parse", "--path-format=absolute", "--git-common-dir"]), "review-eval-"));
  const planted = cases.map((evalCase) => ({ evalCase, ...plantCommit(evalCase, scratch) }));

  for (const { evalCase, sha, line } of planted) {
    process.stdout.write(`\n== ${evalCase.id} (PR #${evalCase.pr})\n`);
    if (evalCase.spec) process.stdout.write(`spec, opened as the issue the PR closes:\n${evalCase.spec}\n`);
    if (!evalCase.plant) {
      process.stdout.write("clean: no plant, any important finding is a candidate false positive\n");
      continue;
    }
    process.stdout.write(`${evalCase.plant.file}:${line}  ${evalCase.plant.regression}\n`);
    process.stdout.write(`${git(["diff", "--unified=1", evalCase.head, sha])}\n`);
  }
  if (!run) {
    rmSync(scratch, { recursive: true });
    process.stdout.write("\nDry run: nothing pushed, nothing spent. Re-run with --run to review the cases.\n");
    return;
  }

  const repo = gh("repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner");
  const schema = execFileSync(
    process.execPath,
    [...process.execArgv, new URL("print-schema.script.ts", import.meta.url).pathname],
    {
      encoding: "utf8",
    },
  );
  const lock = lockedPermissions(pipeline);
  const origin = git(["remote", "get-url", "origin"]);
  const resultsFile = join(scratch, "results.json");
  const outcomes: Outcome[] = [];
  for (const { evalCase, sha, line } of planted) {
    if (remaining < MIN_CASE_BUDGET_USD) {
      process.stdout.write(`\nbudget spent; skipping ${evalCase.id} and the cases after it\n`);
      break;
    }
    // The reviewer works in this directory and can list around it, so the name
    // is neutral. Unique, so two runs side by side never push to or delete each
    // other's branches.
    const dir = mkdtempSync(join(tmpdir(), "pr-"));
    const branch = basename(dir);
    let issueNumber: string | null = null;
    let prNumber: string | null = null;
    try {
      git(["push", "-q", "origin", `${evalCase.base}:refs/heads/${branch}-base`, `${sha}:refs/heads/${branch}`]);
      const original = z
        .object({ title: z.string(), body: z.string() })
        .parse(JSON.parse(gh("pr", "view", String(evalCase.pr), "--json", "title,body")));
      if (evalCase.spec) {
        const issue = gh("issue", "create", "--title", original.title, "--body", evalCase.spec);
        issueNumber = issue.slice(issue.lastIndexOf("/") + 1);
      }
      const url = gh(
        "pr",
        "create",
        "--draft",
        "--base",
        `${branch}-base`,
        "--head",
        branch,
        "--title",
        original.title,
        "--body",
        // The original body restates the requirements, and the issue must be
        // their only route to the reviewer.
        issueNumber ? `Closes #${issueNumber}.` : original.body,
      );
      prNumber = url.slice(url.lastIndexOf("/") + 1);
      checkout(branch, origin, pipeline, dir);
      process.stdout.write(`\nreviewing ${evalCase.id} as #${prNumber}...\n`);
      const cap = Math.min(CASE_BUDGET_USD, remaining);
      const outcome = grade(evalCase, line, review(evalCase, prNumber, dir, repo, schema, lock, cap));
      // A run that died printed no cost, but may have spent up to its cap.
      remaining -= outcome.cost_usd ?? cap;
      outcomes.push(outcome);
      process.stdout.write(`${outcome.verdict}  $${outcome.cost_usd?.toFixed(2) ?? "?"}\n`);
    } catch (error) {
      const message = String(error).slice(0, 300);
      outcomes.push({
        id: evalCase.id,
        verdict: "error",
        cost_usd: null,
        duration_ms: null,
        importants: [],
        refuted_on_plant: [],
        permission_denials: 0,
        error: message,
      });
      process.stdout.write(`error  ${message}\n`);
    } finally {
      // Each step on its own, so one failing never strands the others.
      const cleanups = [
        () =>
          prNumber ? gh("pr", "close", prNumber, "--delete-branch") : git(["push", "-q", "origin", "--delete", branch]),
        () => git(["push", "-q", "origin", "--delete", `${branch}-base`]),
        () => issueNumber && gh("issue", "close", issueNumber),
        () => issueNumber && gh("issue", "delete", issueNumber, "--yes"),
        () => rmSync(dir, { recursive: true, force: true }),
      ];
      for (const cleanup of cleanups) {
        try {
          cleanup();
        } catch (error) {
          process.stderr.write(`cleanup after ${evalCase.id}: ${String(error).slice(0, 300)}\n`);
        }
      }
      // After every case, so a crash later still leaves what was paid for.
      writeFileSync(resultsFile, `${JSON.stringify({ pipeline, outcomes }, null, 2)}\n`);
    }
  }

  const scored = outcomes.filter((outcome) => outcome.verdict === "hit" || outcome.verdict === "miss");
  const hits = scored.filter((outcome) => outcome.verdict === "hit").length;
  const cleanCases = outcomes.filter(
    (outcome) => outcome.verdict === "clean" || outcome.verdict === "false-positive",
  );
  const flagged = cleanCases.filter((outcome) => outcome.verdict === "false-positive").length;
  const errored = outcomes.filter((outcome) => outcome.verdict === "error").length;
  const skipped = planted.length - outcomes.length;
  const spent = outcomes.reduce((sum, outcome) => sum + (outcome.cost_usd ?? 0), 0);
  const tally = [
    `recall ${hits}/${scored.length}`,
    `${flagged}/${cleanCases.length} clean cases flagged`,
    ...(errored ? [`${errored} errored`] : []),
    ...(skipped ? [`${skipped} skipped for budget`] : []),
  ].join(", ");
  process.stdout.write(`\n${tally}, reported spend $${spent.toFixed(2)}; details in ${resultsFile}\n`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
