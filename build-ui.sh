#!/usr/bin/env sh
# 从仓库根目录构建前端，供 `cargo tauri build` 的 beforeBuildCommand 使用。
# 必须在仓库根目录执行（即 cd /path/to/Admiral && cargo tauri build）。
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/crates/admiral-client/ui" && npm run build
