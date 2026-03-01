import { useState, useRef, useEffect } from "react";
import { Link } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { useUiStore, type ConnectionStatus } from "@/stores/uiStore";

const HEADER_HEIGHT = 56;

function StatusBadge({ status }: { status: ConnectionStatus }) {
  const dotColor =
    status === "online"
      ? "var(--color-success)"
      : status === "connecting"
        ? "var(--color-warning)"
        : "#9fb3c8";

  const label =
    status === "online" ? "在线" : status === "connecting" ? "连接中" : "离线";

  const badgeClass =
    status === "online"
      ? "admiral-chip admiral-chip-success"
      : status === "connecting"
        ? "admiral-chip admiral-chip-warning"
        : "admiral-chip";

  return (
    <span className={badgeClass} title={label}>
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

function LogoIcon() {
  return (
    <svg
      width={20}
      height={20}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ flexShrink: 0 }}
      aria-hidden
    >
      <path d="M12 22V8" />
      <path d="M5 12H2a10 10 0 0 0 20 0h-3" />
      <circle cx="12" cy="5" r="3" />
    </svg>
  );
}

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
      aria-hidden
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
      if (ref.current && !ref.current.contains(e.target as Node)) setAccountOpen(false);
    };

    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAccountOpen(false);
    };

    document.addEventListener("click", onOutside);
    document.addEventListener("keydown", onEsc);

    return () => {
      document.removeEventListener("click", onOutside);
      document.removeEventListener("keydown", onEsc);
    };
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
        padding: "0 16px",
        borderBottom: "1px solid var(--admiral-border)",
        background: "linear-gradient(180deg, #ffffff 0%, #f7fbff 100%)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 12, minWidth: 0 }}>
        <Link
          to="/"
          style={{
            display: "inline-flex",
            alignItems: "center",
            gap: 10,
            color: "var(--admiral-text-primary)",
            fontWeight: 700,
            textDecoration: "none",
            minWidth: 0,
          }}
        >
          <span
            style={{
              width: 30,
              height: 30,
              borderRadius: 8,
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#ffffff",
              background: "linear-gradient(135deg, #0f6cbd 0%, #00a3a3 100%)",
              boxShadow: "0 8px 20px rgba(15, 108, 189, 0.25)",
            }}
          >
            <LogoIcon />
          </span>
          <span style={{ letterSpacing: 0.2 }}>Admiral</span>
        </Link>
        {user && <StatusBadge status={connectionStatus} />}
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
        {user && (
          <>
            <div style={{ position: "relative" }} ref={ref}>
              <button
                type="button"
                onClick={() => setAccountOpen((o) => !o)}
                className="btn btn-default"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: 8,
                  paddingLeft: 10,
                  paddingRight: 10,
                  maxWidth: 220,
                }}
                aria-expanded={accountOpen}
                aria-haspopup="true"
              >
                <span
                  style={{
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {user.email}
                </span>
                <span style={{ color: "var(--admiral-text-placeholder)" }} aria-hidden>
                  ▾
                </span>
              </button>

              {accountOpen && (
                <div
                  role="menu"
                  className="header-dropdown"
                  style={{
                    position: "absolute",
                    right: 0,
                    top: "100%",
                    marginTop: 6,
                    padding: "4px 0",
                    width: 220,
                    borderRadius: "var(--admiral-radius-card)",
                    backgroundColor: "var(--admiral-card)",
                    border: "1px solid var(--admiral-border)",
                    boxShadow: "var(--admiral-shadow-raised)",
                    zIndex: 10,
                    animation: "header-dropdown-in 0.15s ease-out",
                  }}
                >
                  <div
                    className="header-dropdown-item"
                    style={{
                      padding: "9px 12px",
                      borderBottom: "1px solid var(--admiral-border)",
                      fontSize: 13,
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
                    className="header-dropdown-item header-dropdown-logout"
                    style={{
                      width: "100%",
                      textAlign: "left",
                      padding: "9px 12px",
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
                width: 34,
                height: 34,
                color: "var(--admiral-text-secondary)",
                borderRadius: 10,
                border: "1px solid var(--admiral-border)",
                background: "#ffffff",
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
