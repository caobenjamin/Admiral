import { useEffect, useMemo, useState, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useTaskStore, type TaskMeta, type TaskStatus } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { Button } from "@/components/common/Button";
import * as tauri from "@/lib/tauri";

function taskStatusLabel(s: TaskStatus): string {
  const map: Record<string, string> = {
    draft: "草稿",
    exploring: "探索中",
    configuring: "配置中",
    testing: "测试中",
    ready: "就绪",
    active: "运行中",
    running: "执行中",
    waiting_for_approval: "等待确认",
    paused: "已暂停",
    completed: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };
  return map[s] ?? s;
}

function statusClass(s: TaskStatus): string {
  if (s === "running" || s === "testing") return "admiral-chip admiral-chip-warning";
  if (s === "completed" || s === "ready") return "admiral-chip admiral-chip-success";
  if (s === "failed" || s === "cancelled") return "admiral-chip admiral-chip-danger";
  if (s === "waiting_for_approval") return "admiral-chip admiral-chip-warning";
  if (s === "active" || s === "configuring") return "admiral-chip admiral-chip-primary";
  return "admiral-chip";
}

function statusDotColor(s: TaskStatus): string {
  if (s === "running" || s === "testing" || s === "waiting_for_approval") return "var(--color-warning)";
  if (s === "completed" || s === "ready") return "var(--color-success)";
  if (s === "failed" || s === "cancelled") return "var(--color-danger)";
  if (s === "active" || s === "configuring") return "var(--color-primary)";
  return "#9fb3c8";
}

function lastRunResultLabel(r: "success" | "failed" | null | undefined): string {
  if (r === "success") return "成功";
  if (r === "failed") return "失败";
  return "暂无";
}

function formatDate(v: string): string {
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return "-";
  return d.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

interface TransitionItem {
  label: string;
  target: TaskStatus;
}

function getTransitions(status: TaskStatus, taskType: string): TransitionItem[] {
  const items: TransitionItem[] = [];
  switch (status) {
    case "draft":
      items.push({ label: "开始探索", target: "exploring" });
      break;
    case "exploring":
      items.push({ label: "进入配置", target: "configuring" });
      break;
    case "configuring":
      items.push({ label: "开始试运行", target: "testing" });
      break;
    case "ready":
      if (taskType !== "instant") items.push({ label: "发布", target: "active" });
      items.push({ label: "执行", target: "running" });
      items.push({ label: "调整配置", target: "configuring" });
      break;
    case "active":
      items.push({ label: "暂停", target: "paused" });
      items.push({ label: "调整配置", target: "configuring" });
      break;
    case "running":
      items.push({ label: "暂停", target: "paused" });
      break;
    case "paused":
      items.push({ label: "继续", target: "running" });
      if (taskType !== "instant") items.push({ label: "恢复调度", target: "active" });
      break;
    case "failed":
      items.push({ label: "重新配置", target: "configuring" });
      break;
    case "completed":
      if (taskType !== "instant") items.push({ label: "重新调度", target: "active" });
      items.push({ label: "调整配置", target: "configuring" });
      break;
    case "cancelled":
      items.push({ label: "重新开始", target: "draft" });
      break;
  }
  return items;
}

// ---- Context Menu ----

interface ContextMenuState {
  taskId: string;
  taskName: string;
  x: number;
  y: number;
  transitions: TransitionItem[];
}

function ContextMenu({
  menu,
  onClose,
  onTransition,
  onDelete,
}: {
  menu: ContextMenuState;
  onClose: () => void;
  onTransition: (taskId: string, target: TaskStatus) => void;
  onDelete: (taskId: string) => Promise<void>;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onCloseRef.current();
    };
    const escHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCloseRef.current();
    };
    document.addEventListener("mousedown", handler);
    document.addEventListener("keydown", escHandler);
    return () => {
      document.removeEventListener("mousedown", handler);
      document.removeEventListener("keydown", escHandler);
    };
  }, []);

  const itemStyle: React.CSSProperties = {
    display: "block",
    width: "100%",
    padding: "7px 14px",
    fontSize: 13,
    textAlign: "left",
    background: "none",
    border: "none",
    cursor: "pointer",
    color: "var(--admiral-text-primary)",
    whiteSpace: "nowrap",
  };

  const handleDeleteClick = useCallback(async () => {
    if (!confirmingDelete) {
      setConfirmingDelete(true);
      return;
    }
    setDeleting(true);
    try {
      await onDelete(menu.taskId);
    } finally {
      setDeleting(false);
    }
  }, [confirmingDelete, onDelete, menu.taskId]);

  return (
    <div
      ref={ref}
      style={{
        position: "fixed",
        left: menu.x,
        top: menu.y,
        zIndex: 9999,
        minWidth: 160,
        background: "var(--admiral-card, #fff)",
        border: "1px solid var(--admiral-border)",
        borderRadius: 8,
        boxShadow: "0 8px 24px rgba(0,0,0,0.15)",
        padding: "4px 0",
      }}
    >
      {menu.transitions.length > 0 && (
        <>
          <div style={{ padding: "4px 14px", fontSize: 11, color: "var(--admiral-text-placeholder)", fontWeight: 600 }}>
            切换状态
          </div>
          {menu.transitions.map((t) => (
            <button
              key={t.target}
              type="button"
              style={itemStyle}
              onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--admiral-hover, #f0f4f8)"; }}
              onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "none"; }}
              onClick={() => {
                onTransition(menu.taskId, t.target);
                onClose();
              }}
            >
              {t.label} → {taskStatusLabel(t.target)}
            </button>
          ))}
          <div style={{ height: 1, background: "var(--admiral-border)", margin: "4px 0" }} />
        </>
      )}
      <button
        type="button"
        disabled={deleting}
        style={{
          ...itemStyle,
          color: confirmingDelete ? "#fff" : "var(--color-danger, #e53e3e)",
          background: confirmingDelete ? "var(--color-danger, #e53e3e)" : "none",
          fontWeight: confirmingDelete ? 600 : 400,
        }}
        onMouseEnter={(e) => {
          if (!confirmingDelete) (e.currentTarget as HTMLElement).style.background = "rgba(229,62,62,0.06)";
        }}
        onMouseLeave={(e) => {
          if (!confirmingDelete) (e.currentTarget as HTMLElement).style.background = "none";
        }}
        onClick={handleDeleteClick}
      >
        {deleting ? "删除中..." : confirmingDelete ? "确认删除？" : "删除任务"}
      </button>
    </div>
  );
}

// ---- TaskListItem ----

function TaskListItem({
  task,
  selected,
  onSelect,
  onContextMenu,
}: {
  task: TaskMeta;
  selected: boolean;
  onSelect: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      onContextMenu={onContextMenu}
      role="option"
      aria-selected={selected}
      className="admiral-page-enter"
      style={{
        width: "100%",
        borderRadius: 12,
        border: selected ? "1px solid rgba(15, 108, 189, 0.5)" : "1px solid var(--admiral-border)",
        background: selected ? "linear-gradient(180deg, #f1f8ff 0%, #eaf4ff 100%)" : "#ffffff",
        padding: "11px 12px",
        textAlign: "left",
        display: "flex",
        flexDirection: "column",
        gap: 8,
        boxShadow: selected ? "0 10px 22px rgba(15, 108, 189, 0.12)" : "var(--admiral-shadow-soft)",
        transition: "border-color 0.16s ease, box-shadow 0.16s ease, transform 0.16s ease",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            flexShrink: 0,
            backgroundColor: statusDotColor(task.status),
          }}
          aria-hidden
        />
        <span
          title={task.name}
          style={{
            flex: 1,
            minWidth: 0,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            fontSize: 14,
            fontWeight: selected ? 700 : 600,
            color: "var(--admiral-text-primary)",
          }}
        >
          {task.name || "未命名任务"}
        </span>
        <span className={statusClass(task.status)}>{taskStatusLabel(task.status)}</span>
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: 8,
          fontSize: 12,
          color: "var(--admiral-text-secondary)",
        }}
      >
        <span>更新于 {formatDate(task.updated_at)}</span>
        <span
          style={{
            color:
              task.last_run_result === "success"
                ? "var(--color-success)"
                : task.last_run_result === "failed"
                  ? "var(--color-danger)"
                  : "var(--admiral-text-placeholder)",
          }}
        >
          {lastRunResultLabel(task.last_run_result)}
        </span>
      </div>
    </button>
  );
}

// ---- TaskList ----

export function TaskList() {
  const navigate = useNavigate();
  const { tasks, fetchTasks } = useTaskStore();
  const { selectedTaskId, setSelectedTaskId } = useUiStore();
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | TaskStatus>("all");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  useEffect(() => {
    fetchTasks();
  }, [fetchTasks]);

  const filteredTasks = useMemo(() => {
    return tasks.filter((t) => {
      const hitStatus = statusFilter === "all" || t.status === statusFilter;
      const hitQuery = !query.trim() || t.name.toLowerCase().includes(query.trim().toLowerCase());
      return hitStatus && hitQuery;
    });
  }, [tasks, query, statusFilter]);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, task: TaskMeta) => {
      e.preventDefault();
      e.stopPropagation();
      const taskType = task.type === "scheduled" ? "cron" : task.type;
      setContextMenu({
        taskId: task.id,
        taskName: task.name,
        x: e.clientX,
        y: e.clientY,
        transitions: getTransitions(task.status, taskType),
      });
    },
    []
  );

  const handleTransition = useCallback(
    async (taskId: string, target: TaskStatus) => {
      try {
        await tauri.taskTransition(taskId, target);
        await fetchTasks();
      } catch (err) {
        console.error("transition failed:", err);
      }
    },
    [fetchTasks]
  );

  const handleDelete = useCallback(
    async (taskId: string) => {
      try {
        await tauri.taskDelete(taskId);
        setContextMenu(null);
        if (selectedTaskId === taskId) {
          setSelectedTaskId(null);
        }
        await fetchTasks();
      } catch (err: unknown) {
        console.error("delete failed:", err);
        alert(`删除失败: ${err instanceof Error ? err.message : String(err)}`);
      }
    },
    [fetchTasks, selectedTaskId, setSelectedTaskId]
  );

  return (
    <div
      className="h-full min-h-0"
      style={{ display: "flex", flexDirection: "column", overflow: "hidden", background: "transparent" }}
      role="listbox"
      aria-label="任务列表"
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderBottom: "1px solid var(--admiral-border)",
          padding: "12px 14px",
          gap: 12,
        }}
      >
        <h3 style={{ fontSize: 15, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>任务列表</h3>
        <Button variant="primary" size="small" onClick={() => navigate("/task/new")}>
          + 新建任务
        </Button>
      </div>

      <div style={{ padding: 12, borderBottom: "1px solid var(--admiral-border)", display: "flex", gap: 8 }}>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索任务"
          className="admiral-field"
          style={{
            flex: 1,
            minWidth: 0,
            height: 32,
            padding: "0 10px",
            outline: "none",
          }}
        />
        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as "all" | TaskStatus)}
          className="admiral-field"
          style={{ height: 32, padding: "0 8px", minWidth: 100, outline: "none" }}
          aria-label="按状态筛选"
        >
          <option value="all">全部状态</option>
          <option value="draft">草稿</option>
          <option value="exploring">探索中</option>
          <option value="configuring">配置中</option>
          <option value="testing">测试中</option>
          <option value="ready">就绪</option>
          <option value="active">运行中</option>
          <option value="running">执行中</option>
          <option value="completed">已完成</option>
          <option value="failed">失败</option>
          <option value="cancelled">已取消</option>
        </select>
      </div>

      <div style={{ flex: 1, overflowY: "auto", padding: 10, display: "flex", flexDirection: "column", gap: 10 }}>
        {filteredTasks.length === 0 ? (
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              padding: "40px 16px",
              textAlign: "center",
            }}
          >
            <span style={{ fontSize: 28, marginBottom: 8 }} aria-hidden>
              📋
            </span>
            <p style={{ margin: 0, fontSize: 14, color: "var(--admiral-text-secondary)" }}>暂无符合条件的任务</p>
            <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--admiral-text-placeholder)" }}>
              调整筛选条件或创建新任务
            </p>
          </div>
        ) : (
          filteredTasks.map((t) => (
            <TaskListItem
              key={t.id}
              task={t}
              selected={selectedTaskId === t.id}
              onSelect={() => setSelectedTaskId(t.id)}
              onContextMenu={(e) => handleContextMenu(e, t)}
            />
          ))
        )}
      </div>

      {contextMenu && (
        <ContextMenu
          menu={contextMenu}
          onClose={() => setContextMenu(null)}
          onTransition={handleTransition}
          onDelete={handleDelete}
        />
      )}
    </div>
  );
}
