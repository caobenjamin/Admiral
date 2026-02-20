#!/usr/bin/env sh
# Tauri 在 crates/ 下执行 beforeDevCommand，故在此用脚本进入 admiral-client/ui 再启动 dev。
set -e
cd "$(dirname "$0")/admiral-client/ui" && npm run dev
