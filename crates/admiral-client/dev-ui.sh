#!/bin/sh
# Resolve script directory so "ui" is found no matter where the command is run from
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/ui" && npm run dev
