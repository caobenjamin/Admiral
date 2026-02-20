import { useState, useRef, useEffect } from "react";
import { Link } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { useUiStore, type ConnectionStatus } from "@/stores/uiStore";

const HEADER_HEIGHT = 48;

function StatusDot({ status }: { status: ConnectionStatus }) {
  const dotColor =
    status === "online"
      ? "var(--color-success)"
      : status === "connecting"
        ? "var(--color-warning)"
        : "#9ca3af";
  const label =
    status === "online" ? "在线" : status === "connecting" ? "连接中" : "离线";
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        fontSize: 14,
        color: "var(--admiral-text-secondary)",
      }}
      title={label}
    >
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          flexShrink: 0,
          backgroundColor: dotColor,
        }}
        aria-hidden
      />
      <span>{label}</span>
    </span>
  );
}

/** Logo 锚点图标：固定 20x20，防止被撑大 */
function LogoIcon() {
  return (
    <svg
      width={20}
      height={20}
      viewBox="0 0 24 24"
      fill="none"
      stroke="var(--color-primary)"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ flexShrink: 0 }}
    >
      <path d="M12 22V8" />
      <path d="M5 12H2a10 10 0 0 0 20 0h-3" />
      <circle cx="12" cy="5" r="3" />
    </svg>
  );
}

/** 设置齿轮图标：固定 16x16 */
function SettingsIcon() {
  return (
    <svg
      width={16}
      height={16}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

export function Header() {
  const user = useAuthStore((s) => s.user);
  const { logout } = useAuth();
  const connectionStatus = useUiStore((s) => s.connectionStatus);
  const [accountOpen, setAccountOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setAccountOpen(false);
    };
    document.addEventListener("click", onOutside);
    return () => document.removeEventListener("click", onOutside);
  }, []);

  return (
    <header
      style={{
        height: HEADER_HEIGHT,
        flexShrink: 0,
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        paddingLeft: 16,
        paddingRight: 16,
        borderBottom: "1px solid var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        boxShadow: "var(--admiral-shadow-card)",
      }}
    >
      {/* 左：Logo + 产品名 + 连接状态，单行不换行 */}
      <div
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 16,
          minWidth: 0,
          flex: "0 1 auto",
        }}
      >
        <Link
          to="/"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 8,
            color: "var(--admiral-text-primary)",
            fontWeight: 600,
            textDecoration: "none",
          }}
        >
          <LogoIcon />
          <span style={{ whiteSpace: "nowrap" }}>Admiral</span>
        </Link>
        {user && (
          <span style={{ flexShrink: 0 }}>
            <StatusDot status={connectionStatus} />
          </span>
        )}
      </div>

      {/* 右：账户下拉 + 设置 */}
      <div
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 8,
          flexShrink: 0,
        }}
      >
        {user && (
          <>
            <div style={{ position: "relative" }} ref={ref}>
              <button
                type="button"
                onClick={() => setAccountOpen((o) => !o)}
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 6,
                  padding: "6px 8px",
                  fontSize: 14,
                  color: "var(--admiral-text-primary)",
                  border: "1px solid var(--admiral-border)",
                  borderRadius: 6,
                  background: "var(--admiral-card)",
                  cursor: "pointer",
                }}
                aria-expanded={accountOpen}
                aria-haspopup="true"
              >
                <span
                  style={{
                    maxWidth: 140,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {user.email}
                </span>
                <span style={{ color: "var(--admiral-text-placeholder)" }} aria-hidden>▾</span>
              </button>
              {accountOpen && (
                <div
                  style={{
                    position: "absolute",
                    right: 0,
                    top: "100%",
                    marginTop: 4,
                    padding: "4px 0",
                    width: 192,
                    borderRadius: "var(--admiral-radius-card)",
                    backgroundColor: "var(--admiral-card)",
                    border: "1px solid var(--admiral-border)",
                    boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
                    zIndex: 10,
                  }}
                >
                  <div
                    style={{
                      padding: "8px 12px",
                      borderBottom: "1px solid var(--admiral-border)",
                      fontSize: 14,
                      color: "var(--admiral-text-secondary)",
                    }}
                  >
                    {user.name || user.email}
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      logout();
                      setAccountOpen(false);
                    }}
                    style={{
                      width: "100%",
                      textAlign: "left",
                      padding: "8px 12px",
                      fontSize: 14,
                      color: "var(--color-danger)",
                      border: "none",
                      background: "none",
                      cursor: "pointer",
                    }}
                  >
                    退出登录
                  </button>
                </div>
              )}
            </div>
            <Link
              to="/settings"
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                width: 32,
                height: 32,
                color: "var(--admiral-text-placeholder)",
              }}
              title="设置"
              aria-label="设置"
            >
              <SettingsIcon />
            </Link>
          </>
        )}
      </div>
    </header>
  );
}
