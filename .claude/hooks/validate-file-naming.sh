#!/usr/bin/env bash
# PreToolUse(Write) hook — blocks file creation with invalid naming conventions.
# Exit 0 = allow, Exit 2 = block with message.
set -euo pipefail

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# No file path — skip
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# Only check .ts/.tsx files
if [[ ! "$FILE_PATH" =~ \.(ts|tsx)$ ]]; then
  exit 0
fi

# Only check project source directories
if [[ ! "$FILE_PATH" =~ /(apps|packages|services)/ ]]; then
  exit 0
fi

# Extract just the filename
FILENAME=$(basename "$FILE_PATH")

# Skip known exceptions
EXCEPTIONS="index.ts|index.tsx|env.ts|main.ts|app.ts|setup.ts|vite-env.d.ts|routeTree.gen.ts"
if [[ "$FILENAME" =~ ^($EXCEPTIONS)$ ]]; then
  exit 0
fi

# Skip .d.ts files
if [[ "$FILENAME" =~ \.d\.ts$ ]]; then
  exit 0
fi

# Skip .config.ts files
if [[ "$FILENAME" =~ \.config\.ts$ ]]; then
  exit 0
fi

# Skip generated directories
if [[ "$FILE_PATH" =~ /generated/ ]]; then
  exit 0
fi

# Skip __root.tsx (TanStack Router)
if [[ "$FILENAME" == "__root.tsx" ]]; then
  exit 0
fi

# Skip route files with $ prefix (TanStack Router dynamic segments)
if [[ "$FILENAME" =~ ^\$ ]]; then
  exit 0
fi

# Skip TanStack Router nested route files (dot-separated segments in the frontend routes directory)
if [[ "$FILE_PATH" =~ apps/acme-web/src/routes/ ]]; then
  exit 0
fi

# Skip test setup files
if [[ "$FILENAME" == "setup.ts" ]]; then
  exit 0
fi

# Valid roles (25). Authoritative taxonomy in docs/conventions/naming.md.
# Each role marks a distinct calling convention or content kind; no plurality
# duplicates, no compound test roles, no domain words.
VALID_ROLES="service|routes|schemas|serializer|types|interface|enums|constants|test|integration\.test|repository|manager|adapter|factory|config|utils|hook|component|store|errors|middleware|client|mock|registry|script"

# Check kebab-case.role.ts pattern
# Filename must be: kebab-case-name.role.ts (or .tsx)
# kebab-case: lowercase letters, digits, hyphens only
if [[ "$FILENAME" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?\.($VALID_ROLES)\.(ts|tsx)$ ]]; then
  exit 0
fi

# If we get here, the filename is invalid
echo "BLOCKED: File name '$FILENAME' does not match the required pattern: kebab-case.role.ts" >&2
echo "" >&2
echo "Examples: my-service.service.ts, message-frame.types.ts, session.adapter.ts" >&2
echo "" >&2
echo "Valid roles (25):" >&2
echo "  Behavioral: service, repository, serializer, middleware, routes, manager," >&2
echo "    factory, client, adapter, registry, config" >&2
echo "  Declarations: types, interface, schemas, enums, constants, errors" >&2
echo "  Frontend: component, hook, store" >&2
echo "  Tests: test, integration.test, mock" >&2
echo "  Utility: utils (pure functions only; cannot import service/manager/repository/client)" >&2
echo "  Entrypoint: script (runnable main() block, invoked via tsx/node from package.json or shell)" >&2
echo "" >&2
echo "Exceptions (no role needed): index.ts, env.ts, main.ts, app.ts, setup.ts," >&2
echo "  *.d.ts, *.config.ts, TanStack Router files (apps/acme-web/src/routes/**, __root.tsx, \$param.tsx)" >&2
echo "" >&2
echo "Full taxonomy + per-role calling conventions: docs/conventions/naming.md" >&2
exit 2
