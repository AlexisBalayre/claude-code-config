import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
 * nothing. With --run it pushes `eval/<id>` branches, opens a draft PR per case
 * (the workflow never reviews a draft), runs /pr-ci-review against it locally
 * the way the workflow invokes it, then closes the PRs and deletes the
 * branches. A case with a `spec` also opens it as an issue the PR closes, which
 * is what spawns the spec reviewer, and deletes the issue afterwards. --pipeline <ref> picks whose `.claude/` reviews (default HEAD), so
 * two pipeline versions can be scored on the same cases.
 */

const CASES_FILE = new URL("../eval/cases.json", import.meta.url);
// Mirrors the workflow's managed allow list. Locally the project's own settings
// still grant more, so the writes the lock denies are denied here by name.
const ALLOWED = [
  "Agent",
  "Skill",
  "Read",
  "Glob",
  "Grep",
  "StructuredOutput",
  "Bash(gh pr diff*)",
  "Bash(gh pr view*)",
  "Bash(gh issue view*)",
  "Bash(git diff *)",
  "Bash(git status*)",
  "Bash(git log*)",
  "Bash(git show*)",
  "Bash(git rev-parse*)",
  "Bash(git rev-list*)",
  "Bash(git merge-base*)",
];
const DENIED = [
  "Edit",
  "Write",
  "NotebookEdit",
  "Bash(git push*)",
  "Bash(git commit*)",
  "Bash(gh pr comment*)",
  "Bash(gh pr review*)",
  "Bash(gh pr merge*)",
  "Bash(gh pr edit*)",
  "Bash(gh pr close*)",
  "Bash(gh api*)",
];
// The workflow's config restore, narrowed to what this repo carries.
const CONFIG_PATHS = [".claude", "CLAUDE.md", "AGENTS.md"];
// A finding this close to the plant is about it: reviewers anchor on the line
// they reason from, which can sit a few lines off the edited one.
const LINE_WINDOW = 10;
const CASE_BUDGET_USD = 5;

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

/** Commits the plant onto the case's head without a checkout; the clean case keeps its head. */
function plantCommit(evalCase: EvalCase, scratch: string): { sha: string; line: number | null } {
  const { plant, head } = evalCase;
  if (!plant) return { sha: head, line: null };
  // Raw, not git(): trimming would drop the file's final newline.
  const original = execFileSync("git", ["show", `${head}:${plant.file}`], { encoding: "utf8" });
  const at = original.indexOf(plant.find);
  if (at === -1 || original.indexOf(plant.find, at + 1) !== -1) {
    throw new Error(`${evalCase.id}: the plant's find string must occur exactly once in ${plant.file}`);
  }
  const blob = git(["hash-object", "-w", "--stdin"], { input: original.replace(plant.find, plant.replace) });
  const mode = git(["ls-tree", head, "--", plant.file]).split(/\s/)[0];
  const env = { ...process.env, GIT_INDEX_FILE: join(scratch, `${evalCase.id}.index`) };
  git(["read-tree", head], { env });
  git(["update-index", "--cacheinfo", `${mode},${blob},${plant.file}`], { env });
  const tree = git(["write-tree"], { env });
  const sha = git(["commit-tree", tree, "-p", head, "-m", `eval: plant ${evalCase.id}`]);
  return { sha, line: original.slice(0, at).split("\n").length };
}

function lineSpan(line: string | null): [number, number] | null {
  const match = /(\d+)(?:\s*[-–]\s*L?(\d+))?/.exec(line ?? "");
  return match ? [Number(match[1]), Number(match[2] ?? match[1])] : null;
}

function onPlant(finding: Pick<Finding, "file" | "line" | "description">, plant: Plant, line: number): boolean {
  const span = lineSpan(finding.line);
  const near =
    finding.file === plant.file && span !== null && span[0] - LINE_WINDOW <= line && line <= span[1] + LINE_WINDOW;
  const text = `${finding.description}\n${"body" in finding ? (finding.body ?? "") : ""}`;
  return near || plant.keywords.some((keyword) => text.includes(keyword));
}

/** Checks the case out the way the workflow leaves the tree: PR head, base-branch config, the PR's copy under .claude-pr/. */
function checkout(sha: string, pipeline: string, scratch: string, id: string): string {
  const dir = join(scratch, id);
  git(["worktree", "add", "--detach", dir, sha]);
  for (const path of CONFIG_PATHS) {
    if (!existsSync(join(dir, path))) continue;
    cpSync(join(dir, path), join(dir, ".claude-pr", path), { recursive: true });
    rmSync(join(dir, path), { recursive: true });
  }
  const archive = execFileSync("git", ["archive", pipeline, ...CONFIG_PATHS]);
  execFileSync("tar", ["-x", "-C", dir], { input: archive });
  return dir;
}

function review(evalCase: EvalCase, prNumber: string, dir: string, repo: string, schema: string, budget: number) {
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
      "--allowedTools",
      ALLOWED.join(","),
      "--disallowedTools",
      DENIED.join(","),
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
  let remaining = budgetAt === -1 ? 10 : Number(args[budgetAt + 1]);
  const cases = z.array(caseSchema).parse(JSON.parse(readFileSync(CASES_FILE, "utf8")));
  const scratch = mkdtempSync(join(tmpdir(), "review-eval-"));
  const planted = cases.map((evalCase) => ({ evalCase, ...plantCommit(evalCase, scratch) }));

  for (const { evalCase, sha, line } of planted) {
    process.stdout.write(`\n== ${evalCase.id} (PR #${evalCase.pr})\n`);
    if (!evalCase.plant) {
      process.stdout.write("clean: no plant, any important finding is a candidate false positive\n");
      continue;
    }
    process.stdout.write(`${evalCase.plant.file}:${line}  ${evalCase.plant.regression}\n`);
    process.stdout.write(`${git(["diff", "--unified=1", evalCase.head, sha])}\n`);
    if (evalCase.spec) process.stdout.write(`spec, opened as the issue the PR closes:\n${evalCase.spec}\n`);
  }
  if (!run) {
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
  const outcomes: Outcome[] = [];
  for (const { evalCase, sha, line } of planted) {
    if (remaining < 1) {
      process.stdout.write(`\nbudget spent; skipping ${evalCase.id} and the cases after it\n`);
      break;
    }
    const branch = `eval/${evalCase.id}`;
    let issueNumber: string | null = null;
    let prNumber: string | null = null;
    let dir: string | null = null;
    try {
      git(["push", "-q", "origin", `${evalCase.base}:refs/heads/${branch}-base`, `${sha}:refs/heads/${branch}`]);
      if (evalCase.spec) {
        const issue = gh("issue", "create", "--title", `eval: ${evalCase.id} spec (do not act)`, "--body", evalCase.spec);
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
        `eval: ${evalCase.id} (do not merge)`,
        "--body",
        `${issueNumber ? `Closes #${issueNumber}.\n\n` : ""}Review eval case replaying #${evalCase.pr}. Opened and closed by review-eval.script.ts.`,
      );
      prNumber = url.slice(url.lastIndexOf("/") + 1);
      dir = checkout(sha, pipeline, scratch, evalCase.id);
      process.stdout.write(`\nreviewing ${evalCase.id} as #${prNumber}...\n`);
      const outcome = grade(
        evalCase,
        line,
        review(evalCase, prNumber, dir, repo, schema, Math.min(CASE_BUDGET_USD, remaining)),
      );
      remaining -= outcome.cost_usd ?? 0;
      outcomes.push(outcome);
      process.stdout.write(`${outcome.verdict}  $${outcome.cost_usd?.toFixed(2) ?? "?"}\n`);
    } finally {
      if (prNumber) gh("pr", "close", prNumber, "--delete-branch");
      else git(["push", "-q", "origin", "--delete", branch]);
      git(["push", "-q", "origin", "--delete", `${branch}-base`]);
      if (issueNumber) gh("issue", "delete", issueNumber, "--yes");
      if (dir) git(["worktree", "remove", "--force", dir]);
    }
  }

  const resultsFile = join(scratch, "results.json");
  writeFileSync(resultsFile, `${JSON.stringify({ pipeline, outcomes }, null, 2)}\n`);
  const planted_ = outcomes.filter((outcome) => outcome.verdict === "hit" || outcome.verdict === "miss");
  const hits = planted_.filter((outcome) => outcome.verdict === "hit").length;
  const spent = outcomes.reduce((sum, outcome) => sum + (outcome.cost_usd ?? 0), 0);
  process.stdout.write(`\nrecall ${hits}/${planted_.length}, spent $${spent.toFixed(2)}; details in ${resultsFile}\n`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
