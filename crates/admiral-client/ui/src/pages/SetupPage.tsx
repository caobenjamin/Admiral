import { Link } from "react-router-dom";

export function SetupPage() {
  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 760 }}>
        <h2 className="settings-title">初始化向导</h2>
        <p className="settings-subtitle">完成权限申请与平台授权后即可开始使用。您可以在设置中随时管理。</p>

        <div
          style={{
            marginTop: 14,
            border: "1px solid var(--admiral-border)",
            borderRadius: 12,
            padding: 14,
            background: "#f7fbff",
          }}
        >
          <p style={{ margin: 0, fontSize: 14, color: "var(--admiral-text-primary)" }}>
            推荐先完成 B 站授权与账户绑定，再开始创建任务和 Agent 对话执行。
          </p>
        </div>

        <div style={{ marginTop: 14, display: "flex", gap: 10 }}>
          <Link to="/settings/platforms" className="btn btn-primary">
            前往平台授权
          </Link>
          <Link to="/" className="btn btn-default">
            稍后处理
          </Link>
        </div>
      </div>
    </div>
  );
}
