#!/bin/sh
set -eu
command -v gitleaks >/dev/null 2>&1 || { echo "Install gitleaks before committing (macOS: brew install gitleaks)." >&2; exit 1; }
gitleaks git --pre-commit --staged --redact --verbose=false
