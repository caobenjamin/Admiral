import { useEffect, useRef, useCallback, useState } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { useUiStore, type ActiveTab } from "@/stores/uiStore";
import { TaskList } from "@/components/task/TaskList";

function TwoColumnLayout({ children }: { children: React.ReactNode }) {
  const { taskListWidth, setTaskListWidth } = useUiStore();
  const [isDragging, setIsDragging] = useState(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  useEffect(() => {
    if (!isDragging) return;

    const onMove = (e: MouseEvent) => {
      const dx = e.clientX - startX.current;
      setTaskListWidth(startWidth.current + dx);
    };

    const onUp = () => {
      setIsDragging(false);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);

    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [isDragging, setTaskListWidth]);

  const onResizeMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsDragging(true);
      startX.current = e.clientX;
      startWidth.current = taskListWidth;
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [taskListWidth]
  );

  return (
    <main
      className="admiral-page-enter"
      style={{
        flex: 1,
        minWidth: 0,
        minHeight: 0,
        overflow: "hidden",
        display: "flex",
        flexDirection: "row",
        padding: 14,
        gap: 0,
      }}
    >
      <div
        className="admiral-surface"
        style={{
          width: taskListWidth,
          minWidth: 240,
          maxWidth: 400,
          flexShrink: 0,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <TaskList />
      </div>

      <div
        role="separator"
        aria-orientation="vertical"
        onMouseDown={onResizeMouseDown}
        className={`resize-handle ${isDragging ? "resize-dragging" : ""}`}
        style={{
          width: 8,
          margin: "0 6px",
          borderRadius: 8,
          flexShrink: 0,
          cursor: "col-resize",
        }}
      />

      <div
        className="admiral-surface"
        style={{
          flex: 1,
          minWidth: 480,
          minHeight: 0,
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {children}
      </div>
    </main>
  );
}

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
          padding: 16,
        }}
      >
        <div
          className="admiral-surface"
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
      <TwoColumnLayout>
        <Outlet />
      </TwoColumnLayout>
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
        padding: 14,
      }}
    >
      <div className="admiral-surface" style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
        <Outlet />
      </div>
    </main>
  );
}
