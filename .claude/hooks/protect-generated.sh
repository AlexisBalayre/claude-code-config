#!/bin/bash
# Protect generated files from manual edits
# Runs as PreToolUse hook on Edit/Write tool calls

set -o pipefail

# The tool input is passed via stdin as JSON
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | grep -o '"file_path":"[^"]*"' | head -1 | sed 's/"file_path":"//;s/"$//')

# If we can't extract the file path, allow it
if [ -z "$FILE_PATH" ]; then
  exit 0
fi

# Block: any *.gen.ts file
if echo "$FILE_PATH" | grep -qE '\.gen\.ts$'; then
  echo "BLOCKED: *.gen.ts files are auto-generated. Do not edit manually." >&2
  exit 2
fi

# Block: gRPC generated stubs
if echo "$FILE_PATH" | grep -qE 'packages/acme-rpc/src/generated/'; then
  echo "BLOCKED: gRPC stubs are auto-generated. Edit the .proto files in packages/acme-rpc/proto/ and run 'pnpm --filter @acme/acme-rpc proto:gen' instead." >&2
  exit 2
fi

exit 0
