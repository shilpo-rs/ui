#!/usr/bin/env bash
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
CHECK_TITLE="$ROOT/.github/scripts/check-pr-title.sh"
"$CHECK_TITLE" "feat(m3e): add component"
"$CHECK_TITLE" "fix!: mark a breaking correction"
"$CHECK_TITLE" "docs(release/contracts): clarify tags"
! "$CHECK_TITLE" "Add component" >/dev/null 2>&1
! "$CHECK_TITLE" "Feat: uppercase type" >/dev/null 2>&1
echo "release contract tests passed"

