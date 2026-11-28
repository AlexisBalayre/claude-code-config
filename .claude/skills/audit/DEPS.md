# Deps mode

Find the dependency debt CI lets through. The project's dependency-rule config holds the rules: CI fails on the ones that encode a written convention (package entry points, the layer rules of `docs/conventions/`). Cycles and orphans break no written convention, so this mode reads them as warnings that never fail CI.

<!-- TODO(adapt): name the dependency-rule tool, its config file, the command that runs it and the switch that turns the cycle and orphan warnings on (for example `dependency-cruiser`, `import-linter`, `go vet` with a cycle check). -->

## 1. Run

Run the dependency check with its cycle and orphan warnings on, narrowed to the scope. Then read the config's listed exceptions, each with the comment that names why it is allowed. With no such tool configured, say which would help and stop.

Done when you hold every warning in the scope and every listed exception. A run can follow imports into other packages: keep only the warnings whose modules all sit in the scope.

## 2. Check each candidate

- **A cycle:** a tool can report one cycle per entry module, so merge the cycles that share a module into one set, one row per set. Read both ends of each edge. A type-only edge (a types file and the module it describes) is cheap to cut; a runtime cycle through a barrel or package index can break module init order and ranks higher.
- **An orphan:** a file nothing imports. It is dead code unless a tool loads it by path (a build or test config, a manifest script, a shell script): grep for its file name.
- **An exception:** check whether its reason still holds, such as a frozen package that has thawed.

Done when each row has its kind, the paths, the evidence and the fix: cut the edge, remove the file, or retire the exception.

## 3. Pick sheet and file

Build the `html-artifact` pick sheet with the choices `file` and `skip`. File the picked rows as the shared rules say: type `Improvement` for a cycle, `Chore` for an orphan or an exception.
