#!/usr/bin/env sh
# 从仓库根目录启动前端 dev server，供 `cargo tauri dev` 的 beforeDevCommand 使用。
# 必须在仓库根目录执行本脚本（即 cd /path/to/Admiral && cargo tauri dev）。
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/crates/admiral-client/ui" && npm run dev
