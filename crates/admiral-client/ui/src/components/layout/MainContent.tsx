import { useEffect } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { useUiStore, type ActiveTab } from "@/stores/uiStore";
import { TaskList } from "@/components/task/TaskList";

function useSyncActiveTabFromRoute() {
  const location = useLocation();
  const setActiveTab = useUiStore((s) => s.setActiveTab);

  useEffect(() => {
    const path = location.pathname;
    let tab: ActiveTab = "tasks";
    if (path.startsWith("/settings")) tab = "media";
    else if (path === "/" || path.startsWith("/task/new")) tab = "tasks";
    setActiveTab(tab);
  }, [location.pathname, setActiveTab]);
}

export function MainContent() {
  const activeTab = useUiStore((s) => s.activeTab);
  const location = useLocation();

  useSyncActiveTabFromRoute();

  const isSingleColumn =
    location.pathname.startsWith("/settings") ||
    location.pathname.startsWith("/task/new") ||
    location.pathname === "/setup" ||
    location.pathname === "/admin";

  const showTwoColumn = location.pathname === "/";
  const showPlaceholder = !showTwoColumn && !isSingleColumn;

  if (showPlaceholder) {
    return (
      <main
        style={{
          flex: 1,
          minWidth: 0,
          minHeight: 0,
          overflow: "auto",
          display: "flex",
          flexDirection: "column",
          backgroundColor: "var(--admiral-bg)",
        }}
      >
        <div
          style={{
            flex: 1,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: 32,
            color: "var(--admiral-text-placeholder)",
            fontSize: 14,
          }}
        >
          {activeTab === "skills" && <p>能力栏功能将在 Phase 5 开放</p>}
          {activeTab === "other" && <p>敬请期待</p>}
        </div>
      </main>
    );
  }

  if (showTwoColumn) {
    return (
      <main
        style={{
          flex: 1,
          minWidth: 0,
          minHeight: 0,
          overflow: "hidden",
          display: "flex",
          flexDirection: "row",
          backgroundColor: "var(--admiral-bg)",
        }}
      >
        <div
          style={{
            width: 300,
            flexShrink: 0,
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
            borderRight: "1px solid var(--admiral-border)",
            backgroundColor: "var(--admiral-card)",
          }}
        >
          <TaskList />
        </div>
        <div
          style={{
            flex: 1,
            minWidth: 0,
            minHeight: 0,
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          <Outlet />
        </div>
      </main>
    );
  }

  return (
    <main
      style={{
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        overflow: "auto",
        display: "flex",
        flexDirection: "column",
        backgroundColor: "var(--admiral-bg)",
      }}
    >
      <Outlet />
    </main>
  );
}
