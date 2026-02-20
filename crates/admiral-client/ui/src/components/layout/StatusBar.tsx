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
  const statusLabel = connectionStatus === "online" ? "在线" : connectionStatus === "connecting" ? "连接中" : "离线";
  const dotColor =
    connectionStatus === "online"
      ? "var(--color-success)"
      : connectionStatus === "connecting"
        ? "var(--color-warning)"
        : "#9CA3AF";

  return (
    <footer
      style={{
        height: 28,
        flexShrink: 0,
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        paddingLeft: 16,
        paddingRight: 16,
        borderTop: "1px solid var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        color: "var(--admiral-text-secondary)",
        fontSize: 12,
      }}
    >
      <div style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 16 }}>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <span
            style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              flexShrink: 0,
              backgroundColor: dotColor,
            }}
            aria-hidden
          />
          {statusLabel}
        </span>
        {license?.is_active && days !== null && (
          <span>License: 剩余{days}天</span>
        )}
      </div>
      <span>Admiral v1.0.0</span>
    </footer>
  );
}
