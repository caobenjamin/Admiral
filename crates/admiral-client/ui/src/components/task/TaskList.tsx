import { useEffect } from "react";
import { Link } from "react-router-dom";
import { useTaskStore, type TaskMeta, type TaskStatus } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";

function taskStatusLabel(s: TaskStatus): string {
  const map: Record<string, string> = {
    draft: "草稿",
    planning: "规划中",
    plan_ready: "计划就绪",
    dry_running: "Dry Run",
    dry_run_passed: "Dry Run 通过",
    test_running: "测试中",
    test_run_passed: "测试通过",
    deployed: "已部署",
    running: "运行中",
    scheduled: "已排期",
    paused: "已暂停",
    succeeded: "已完成",
    failed: "失败",
  };
  return map[s] ?? s;
}

function taskStatusDot(s: TaskStatus): string {
  if (s === "running" || s === "test_running" || s === "dry_running") return "bg-warning";
  if (s === "succeeded") return "bg-success";
  if (s === "failed") return "bg-danger";
  if (s === "scheduled") return "bg-primary";
  return "bg-gray-400";
}

function lastRunResultLabel(r: "success" | "failed" | null | undefined): string {
  if (r === "success") return "成功";
  if (r === "failed") return "失败";
  return "暂无";
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
      className={`block w-full text-left px-3 py-2.5 rounded-lg flex flex-col gap-1.5 transition-colors duration-150 shrink-0 ${
        selected
          ? "bg-[rgba(64,158,255,0.1)] text-[var(--color-primary)]"
          : "border border-[var(--admiral-border)] bg-[var(--admiral-card)] hover:bg-[rgba(0,0,0,0.04)] hover:border-[var(--admiral-text-secondary)] text-[var(--admiral-text-primary)]"
      }`}
      style={{
        borderRadius: "var(--admiral-radius-card)",
        boxShadow: selected ? "0 1px 3px rgba(0,0,0,0.06)" : undefined,
        ...(selected ? { borderLeft: "3px solid var(--color-primary)" } : {}),
      }}
    >
      <div className="flex items-center gap-2">
        <span
          className={`h-2 w-2 shrink-0 rounded-full ${taskStatusDot(task.status)}`}
          style={{ borderRadius: "50%" }}
          aria-hidden
        />
        <span className="flex-1 min-w-0 truncate text-sm font-medium" title={task.name}>
          {task.name || "未命名任务"}
        </span>
      </div>
      <div className="flex items-center gap-1 pl-4 text-xs text-[var(--admiral-text-secondary)]">
        <span>{taskStatusLabel(task.status)}</span>
        {task.last_run_result && (
          <>
            <span>·</span>
            <span
              className={
                task.last_run_result === "success"
                  ? "text-[var(--color-success)]"
                  : task.last_run_result === "failed"
                    ? "text-[var(--color-danger)]"
                    : ""
              }
            >
              {lastRunResultLabel(task.last_run_result)}
            </span>
          </>
        )}
      </div>
    </button>
  );
}

export function TaskList() {
  const { tasks, fetchTasks } = useTaskStore();
  const { selectedTaskId, setSelectedTaskId } = useUiStore();

  useEffect(() => {
    fetchTasks();
  }, [fetchTasks]);

  return (
    <div
      className="h-full min-h-0"
      style={{ display: "flex", flexDirection: "column", overflow: "hidden" }}
      role="listbox"
      aria-label="任务列表"
    >
      <div className="flex items-center justify-between px-3 py-3 border-b shrink-0" style={{ borderColor: "var(--admiral-border)" }}>
        <span className="text-sm font-medium text-[var(--admiral-text-primary)]">
          任务列表
        </span>
        <Link
          to="/task/new"
          className="inline-flex items-center gap-1 px-2.5 py-1.5 text-sm font-medium text-[var(--color-primary)] hover:bg-[rgba(64,158,255,0.1)] rounded transition-colors"
          style={{ borderRadius: "var(--admiral-radius-button)" }}
        >
          + 新建任务
        </Link>
      </div>
      <div
        className="flex-1 overflow-y-auto p-2 min-h-0"
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 8,
        }}
      >
        {tasks.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
            <span className="text-2xl mb-2" aria-hidden>📋</span>
            <p className="text-sm text-[var(--admiral-text-secondary)] mb-1">暂无任务</p>
            <p className="text-xs text-[var(--admiral-text-placeholder)]">点击上方按钮创建第一个任务</p>
          </div>
        ) : (
          tasks.map((t) => (
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
