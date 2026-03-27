import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTaskStore, type TaskMeta, type TaskStatus } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { Button } from "@/components/common/Button";

function taskStatusLabel(s: TaskStatus): string {
  const map: Record<string, string> = {
    draft: "草稿",
    planning: "规划中",
    plan_ready: "计划就绪",
    scheduled: "已排期",
    running: "运行中",
    waiting_for_approval: "等待确认",
    paused: "已暂停",
    succeeded: "已完成",
    partially_succeeded: "部分完成",
    failed: "失败",
    cancelled: "已取消",
  };
  return map[s] ?? s;
}

function statusClass(s: TaskStatus): string {
  if (s === "running") return "admiral-chip admiral-chip-warning";
  if (s === "succeeded") return "admiral-chip admiral-chip-success";
  if (s === "failed" || s === "cancelled") return "admiral-chip admiral-chip-danger";
  if (s === "waiting_for_approval" || s === "partially_succeeded") return "admiral-chip admiral-chip-warning";
  if (s === "scheduled" || s === "plan_ready") return "admiral-chip admiral-chip-primary";
  return "admiral-chip";
}

function statusDotColor(s: TaskStatus): string {
  if (s === "running" || s === "waiting_for_approval") return "var(--color-warning)";
  if (s === "succeeded") return "var(--color-success)";
  if (s === "failed" || s === "cancelled") return "var(--color-danger)";
  if (s === "partially_succeeded") return "var(--color-warning)";
  if (s === "scheduled" || s === "plan_ready") return "var(--color-primary)";
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

function TaskListItem({
  task,
  selected,
  onSelect,
}: {
  task: TaskMeta;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
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

export function TaskList() {
  const navigate = useNavigate();
  const { tasks, fetchTasks } = useTaskStore();
  const { selectedTaskId, setSelectedTaskId } = useUiStore();
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | TaskStatus>("all");

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
          <option value="running">运行中</option>
          <option value="waiting_for_approval">等待确认</option>
          <option value="partially_succeeded">部分完成</option>
          <option value="succeeded">已完成</option>
          <option value="failed">失败</option>
          <option value="scheduled">已排期</option>
          <option value="draft">草稿</option>
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
            />
          ))
        )}
      </div>
    </div>
  );
}
