#!/usr/bin/env sh
# Tauri 在 crates/ 下执行 beforeBuildCommand，故在此用脚本进入 admiral-client/ui 再构建。
set -e
cd "$(dirname "$0")/admiral-client/ui" && npm run build
