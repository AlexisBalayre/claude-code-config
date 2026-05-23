#!/usr/bin/env bash
set -euo pipefail

cat << 'EOF'
{
  "additionalContext": "PRESERVE ACROSS COMPACTION:\n1. Current git branch and worktree path\n2. List of all modified files in this session\n3. Test commands run and their exact results\n4. Any error messages or failing test output"
}
EOF

exit 0
