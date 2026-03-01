export function Preferences() {
  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 760 }}>
        <h2 className="settings-title">偏好设置</h2>
        <p className="settings-subtitle">主题、语言等设置将在后续版本完善。</p>

        <div style={{ marginTop: 14, display: "grid", gap: 12 }}>
          <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, padding: 14, background: "#ffffff" }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)" }}>界面主题</div>
            <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>
              当前版本默认使用浅色主题，后续支持自定义主题方案。
            </p>
          </div>
          <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, padding: 14, background: "#ffffff" }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)" }}>语言与区域</div>
            <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>
              当前版本默认中文，后续将支持多语言切换与时间格式配置。
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
