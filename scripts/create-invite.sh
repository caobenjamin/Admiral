#!/usr/bin/env bash
# 在数据库中直接插入一条邀请码（无需管理员账号）。
# 用法: 从项目根目录执行 ./scripts/create-invite.sh
# 可选环境变量: ADMIRAL_DATABASE_URL，未设置时从 .env 读取。

set -e
cd "$(dirname "$0")/.."

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi
DATABASE_URL="${ADMIRAL_DATABASE_URL:-postgres://admiral:admiral@localhost:5432/admiral}"

# 生成格式与服务端一致的邀请码: INV-XXXXXXXX (8 位大写十六进制)
CODE="INV-$(uuidgen | tr -d '-' | cut -c1-8 | tr 'a-f' 'A-F')"

psql "$DATABASE_URL" -t -c "
  INSERT INTO invites (id, code, trial_days, max_uses, used_count, created_by, expires_at, created_at)
  VALUES (gen_random_uuid(), '$CODE', 7, 1, 0, NULL, NULL, NOW());
"

echo ""
echo "已生成邀请码: $CODE"
echo "试用天数: 7  可用次数: 1"
