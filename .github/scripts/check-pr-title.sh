#!/usr/bin/env bash
set -euo pipefail
title=${1:-${PR_TITLE:-}}
pattern='^(feat|fix|perf|refactor|docs|test|build|ci|chore|style|revert)(\([a-z0-9][a-z0-9._/-]*\))?(!)?: .+'
[[ $title =~ $pattern ]] || { echo "PR title must use Conventional Commit style (for example: feat(m3e): add split button)" >&2; exit 1; }

