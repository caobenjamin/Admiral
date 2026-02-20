/**
 * 管理页面：仅管理员可访问
 * 用户管理、邀请码管理
 * 需后端 JWT role 校验
 */
export function AdminPage() {
  return (
    <div className="p-6 max-w-2xl">
      <h2 className="text-lg font-semibold mb-4" style={{ color: "var(--admiral-text-primary)" }}>
        管理
      </h2>
      <p className="text-sm" style={{ color: "var(--admiral-text-secondary)" }}>
        用户管理、邀请码管理。需管理员权限，Phase 5 接入完整 API。
      </p>
    </div>
  );
}
