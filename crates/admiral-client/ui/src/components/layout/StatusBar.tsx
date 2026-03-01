import { useAuthStore } from "@/stores/authStore";
import { useUiStore } from "@/stores/uiStore";

function licenseRemainingDays(expiresAt: string | null): number | null {
  if (!expiresAt) return null;
  const end = new Date(expiresAt).getTime();
  const now = Date.now();
  const days = Math.ceil((end - now) / (24 * 60 * 60 * 1000));
  return days > 0 ? days : 0;
}

export function StatusBar() {
  const license = useAuthStore((s) => s.license);
  const connectionStatus = useUiStore((s) => s.connectionStatus);

  const days = license?.expires_at ? licenseRemainingDays(license.expires_at) : null;
  const statusLabel =
    connectionStatus === "online" ? "在线" : connectionStatus === "connecting" ? "连接中" : "离线";
  const statusClass =
    connectionStatus === "online"
      ? "admiral-chip admiral-chip-success"
      : connectionStatus === "connecting"
        ? "admiral-chip admiral-chip-warning"
        : "admiral-chip";

  return (
    <footer
      style={{
        height: 32,
        flexShrink: 0,
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "0 16px",
        borderTop: "1px solid var(--admiral-border)",
        background: "linear-gradient(180deg, #f8fbff 0%, #f2f7fb 100%)",
        color: "var(--admiral-text-secondary)",
        fontSize: 12,
      }}
    >
      <div style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 10 }}>
        <span className={statusClass}>{statusLabel}</span>
        {license?.is_active && days !== null && <span className="admiral-chip">License: 剩余{days}天</span>}
      </div>
      <span style={{ color: "var(--admiral-text-placeholder)" }}>Admiral v1.0.0</span>
    </footer>
  );
}
