/**
 * 管理页面：仅管理员可访问
 * 用户管理、邀请码管理
 * 需后端 JWT role 校验
 */
export function AdminPage() {
  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 760 }}>
        <h2 className="settings-title">管理</h2>
        <p className="settings-subtitle">用户管理、邀请码管理。需管理员权限，Phase 5 接入完整 API。</p>
      </div>
    </div>
  );
}
