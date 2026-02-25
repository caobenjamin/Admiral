import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useUiStore } from "@/stores/uiStore";
import * as tauri from "@/lib/tauri";

const TAB_WIDTH_EXPANDED = 220;
const TAB_WIDTH_COLLAPSED = 56;
const ICON_SIZE = 20;
const CHEVRON_SIZE = 16;

function MediaIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} viewBox="0 0 24 24" fill="currentColor" style={{ flexShrink: 0 }}>
      <path d="M17.813 4.653h.847c1.088 0 1.98.893 1.98 1.98v10.734c0 1.088-.892 1.98-1.98 1.98H5.345c-1.088 0-1.98-.892-1.98-1.98V6.633c0-1.087.892-1.98 1.98-1.98h.847v-.017c0-.762.618-1.38 1.38-1.38h1.668c.762 0 1.38.618 1.38 1.38v.017h4.958v-.017c0-.762.618-1.38 1.38-1.38h1.668c.762 0 1.38.618 1.38 1.38v.017zm-4.646 9.346v-2.5l2.5 1.25-2.5 1.25z" />
    </svg>
  );
}

function TasksIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} fill="none" stroke="currentColor" viewBox="0 0 24 24" style={{ flexShrink: 0 }}>
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
    </svg>
  );
}

function SkillsIcon() {
  return (
    <svg width={ICON_SIZE} height={ICON_SIZE} fill="none" stroke="currentColor" viewBox="0 0 24 24" style={{ flexShrink: 0 }}>
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
    </svg>
  );
}

function ChevronLeftIcon() {
  return (
    <svg width={CHEVRON_SIZE} height={CHEVRON_SIZE} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
    </svg>
  );
}

function ChevronRightIcon() {
  return (
    <svg width={CHEVRON_SIZE} height={CHEVRON_SIZE} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
    </svg>
  );
}

export function LeftTab() {
  const { activeTab, setActiveTab, leftTabCollapsed, setLeftTabCollapsed } = useUiStore();
  const navigate = useNavigate();
  const [biliAuthorized, setBiliAuthorized] = useState(false);

  useEffect(() => {
    tauri.oauthStatus().then((list) => setBiliAuthorized(list.length > 0)).catch(() => setBiliAuthorized(false));
  }, []);

  const width = leftTabCollapsed ? TAB_WIDTH_COLLAPSED : TAB_WIDTH_EXPANDED;

  const handleMediaClick = () => {
    setActiveTab("media");
    navigate("/settings/platforms");
  };

  const baseBtnStyle = (active: boolean, disabled?: boolean): React.CSSProperties => ({
    display: "flex",
    alignItems: "center",
    gap: 12,
    padding: "10px 12px",
    margin: "0 8px",
    textAlign: "left",
    fontSize: 14,
    border: "none",
    borderRadius: "var(--admiral-radius-button)",
    cursor: disabled ? "not-allowed" : "pointer",
    backgroundColor: active ? "rgba(64, 158, 255, 0.1)" : "transparent",
    color: active ? "var(--color-primary)" : "var(--admiral-text-primary)",
    textDecoration: "none",
    position: "relative",
    ...(active ? { borderLeft: "3px solid var(--color-primary)", paddingLeft: 9 } : {}),
    ...(disabled ? { opacity: 0.6 } : {}),
  });

  return (
    <aside
      role="navigation"
      aria-label="主导航"
      style={{
        width,
        flexShrink: 0,
        display: "flex",
        flexDirection: "column",
        borderRight: "1px solid var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        transition: "width 0.2s ease",
      }}
    >
      <nav
        style={{
          display: "flex",
          flexDirection: "column",
          flex: 1,
          paddingTop: 8,
          paddingBottom: 8,
        }}
      >
        <button
          type="button"
          onClick={handleMediaClick}
          style={baseBtnStyle(activeTab === "media")}
          title={leftTabCollapsed ? "媒体账户" : undefined}
          className={`left-tab-item ${activeTab === "media" ? "left-tab-active" : ""}`}
        >
          <MediaIcon />
          {!leftTabCollapsed && (
            <div style={{ minWidth: 0, flex: 1 }}>
              <span style={{ display: "block", fontWeight: 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>媒体账户</span>
              <span
                style={{
                  display: "block",
                  fontSize: 12,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  color: biliAuthorized ? "var(--color-success)" : "var(--color-warning)",
                }}
              >
                B站 {biliAuthorized ? "已授权 ✓" : "未授权"}
              </span>
            </div>
          )}
        </button>

        <Link
          to="/"
          onClick={() => setActiveTab("tasks")}
          style={{ ...baseBtnStyle(activeTab === "tasks"), marginTop: 4 }}
          title={leftTabCollapsed ? "任务" : undefined}
          className={`left-tab-item ${activeTab === "tasks" ? "left-tab-active" : ""}`}
        >
          <TasksIcon />
          {!leftTabCollapsed && <span style={{ fontWeight: 500 }}>任务</span>}
        </Link>

        <button
          type="button"
          onClick={() => setActiveTab("skills")}
          disabled
          style={{ ...baseBtnStyle(false, true), marginTop: 4, color: "var(--admiral-text-placeholder)" }}
          title={leftTabCollapsed ? "能力栏（Phase 5）" : undefined}
          className="left-tab-item"
        >
          <SkillsIcon />
          {!leftTabCollapsed && <span style={{ fontWeight: 500 }}>能力栏</span>}
        </button>
      </nav>

      <button
        type="button"
        onClick={() => setLeftTabCollapsed(!leftTabCollapsed)}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          margin: "0 8px 8px",
          padding: 8,
          border: "none",
          borderRadius: "var(--admiral-radius-button)",
          backgroundColor: "transparent",
          color: "var(--admiral-text-secondary)",
          cursor: "pointer",
        }}
        title={leftTabCollapsed ? "展开" : "收起"}
      >
        {leftTabCollapsed ? <ChevronRightIcon /> : <ChevronLeftIcon />}
      </button>
    </aside>
  );
}
