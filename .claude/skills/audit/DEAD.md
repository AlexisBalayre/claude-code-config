# Dead mode

Find code nothing reaches. The scope is a package, a service or the whole repo; ask when the user named none.

## 1. Files and exports: the detector

<!-- TODO(adapt): name the project's dead-code detector and its command here (for example `knip`, `vulture`, `staticcheck`, `cargo check`), and where its config lives. -->

Run the dead-code detector the project configures (check the repo root and the lint config), in its production mode too when it has one: that mode also reports exports only tests call. With no detector, grep per candidate: a source file (extensions in `SOURCE_EXTENSIONS` from `.claude/project.env`) no other file imports by module path, relative path or bare basename; a public symbol with no `\b<symbol>\b` hit from the repo root, other packages included.

Done when the detector's reports, or the grep results, over the scope are in hand.

## 2. Dead feature flags

A detector cannot see a flag in the project's flag store (a config file, a seed script, a database table, a flag service) whose key no code checks.

1. List the keys from wherever the project defines them; ask the user when there is no obvious source.
2. `git grep -n '"<key>"'` over the source roots, migrations left out: the bare quoted key catches every flag-check call alike.
3. A key with no hit, or whose only hit is its definition, is a candidate.

Done when every flag key has its grep result.

## 3. Check each candidate

Check each against these false-positive sources and name the one that applies:

- **String-keyed registries:** anything resolved by name (plugins, providers, handlers), so the detector can report an export only the registry reaches.
- **Framework conventions:** files loaded by path (routes, migrations, schema files, fixtures) that nothing imports.
- **Tooling entry points:** test setup, seed scripts, eval harnesses, CLI entry points.
- **Build-time references:** a build config, a CI workflow, a Dockerfile, a shell script under `scripts/`.
- **Dynamic loading:** template-string imports, reflection by name, plugin entry points.
- **Generated code:** paths matching `GENERATED_PATHS_REGEX` in `.claude/project.env`; regenerate, never report.
- **Flags awaiting rollout:** the flag lands before the code. Check the last 4 weeks of `git log` for the key.

Done when each candidate has its kind (detector or flag), its path or key, when it was last touched, why it is suspect, and a confidence: `High` when no source applies, `Medium` or `Low` otherwise.

## 4. Pick sheet and file

Build the `html-artifact` pick sheet with the choices `file` and `skip`, the rows ranked by confidence and the evidence beside each. File the picked rows as the shared rules say: type `Chore`, one ticket per kind, since a removal PR never mixes detector and flag removals.
