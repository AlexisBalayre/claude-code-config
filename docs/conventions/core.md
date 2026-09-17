# Core Conventions

Universal rules for every source file. Area docs in this folder inherit from here; they never relax these rules.

**Genre contract:** obligations only (rules, gotchas, decision rules, naming). System description lives in `docs/reference/`.

<!-- TODO(adapt): sections marked TODO(adapt) are placeholders; replace each with this project's rules (read 5-10 representative files first) or delete the section. The language-agnostic sections (Altitude, Comments) apply as-is. -->

## Altitude / YAGNI

Build the smallest thing that meets the requirement. Prefer a deep module (simple surface, logic hidden) over several shallow ones; a module too thin to justify its file gets folded into its caller.

- **Inline by default.** No new file, helper, wrapper, param, option, interface, or generic with a single caller, unless a second caller exists today or a convention in this folder prescribes the construct. A one-call `formatX` helper or one-field `options` object is the bloat this targets.
- **No unreachable defensiveness.** No guard, `catch`, or fallback for a state the types or surrounding code already guarantee. Test: if you cannot write the input that reaches the branch, delete it. Real error handling at seams (I/O, external calls, user input) is exempt.
- **Delete when replacing.** No shims, no "removed" markers, no backwards-compat re-exports.

## Comments

Default to no inline comment. A comment explains **WHY** (invariant, unit, ordering, gotcha), never WHAT. A comment whose removal wouldn't confuse a future reader should not exist.

```
retries += 1  # increment the retry counter           (BAD: restates the code)
retries += 1  # 429s are transient, so retry first    (GOOD: explains the why)
```

**Seam-duplication test:** a call-site WHY comment that restates the callee's doc comment is a duplicate; the explanation lives at the seam. Before keeping a comment that explains another module's behaviour, read that module's docs; if they already say it, delete the call-site copy.

Doc comments (docstrings, JSDoc, `///`): an imperative one-line summary is the contract. Never paraphrase the symbol name or a parameter's type; no file-header banners, section dividers, changelog comments, or commented-out code.

<!-- TODO(adapt): state where doc comments are required (e.g. public package surface only) and the doc-comment format. -->

This section is deliberately mirrored in `AGENTS.md`; keep both in sync.

## Language and types

<!-- TODO(adapt): language/type-system rules the linter doesn't enforce, e.g. explicit return types on exports, unions over enums, no untyped escape hatches (`any`, `interface{}`, `# type: ignore`) without a justification comment. -->

## Errors

<!-- TODO(adapt): where custom error types live, the error-handling pattern per layer (raise/throw vs result values), and how errors map to responses. -->

## Logging

- Never log secrets or PII (passwords, tokens, cookies, emails, names). Identify actors by opaque IDs.
- Structured logs: one event = one line, with stable snake_case join keys.

<!-- TODO(adapt): the logger to use and how to build it, required fields, sampling rules for high-volume paths. -->

## Structure

- **No hardcoded values**: ports, URLs, timeouts, pool sizes and rates come from configuration, not literals.

<!-- TODO(adapt): module/package layout rules, dependency direction (what may import what), where config and shared types live. -->

## Naming

<!-- TODO(adapt): file naming (and whether .claude/hooks/validate-file-naming.sh enforces it via FILE_NAMING_* in .claude/project.env), identifier shapes, banned names and what to use instead. -->
