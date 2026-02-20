import { useEffect } from "react";
import { Link } from "react-router-dom";
import { useTaskStore, type TaskMeta, type TaskStatus } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";

function taskStatusLabel(s: TaskStatus): string {
  const map: Record<TaskStatus, string> = {
    draft: "草稿",
    dry_running: "Dry Run",
    dry_run_passed: "Dry Run 通过",
    test_run_ready: "待测试",
    test_run_running: "测试中",
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
  if (s === "running" || s === "test_run_running" || s === "dry_running") return "bg-warning";
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
      className={`w-full text-left px-3 py-2.5 rounded-lg flex flex-col gap-0.5 transition-colors ${
        selected ? "bg-primary/10 text-primary" : "hover:bg-[var(--admiral-bg)] text-[var(--admiral-text-primary)]"
      }`}
      style={{ borderRadius: "var(--admiral-radius-card)" }}
    >
      <div className="flex items-center gap-2">
        <span
          className={`h-2 w-2 shrink-0 rounded-full ${taskStatusDot(task.status)}`}
          aria-hidden
        />
        <span className="flex-1 min-w-0 truncate text-sm font-medium" title={task.name}>
          {task.name || "未命名任务"}
        </span>
      </div>
      <div className="flex items-center justify-between pl-4 text-xs text-[var(--admiral-text-secondary)]">
        <span>{taskStatusLabel(task.status)}</span>
        <span>{lastRunResultLabel(task.last_run_result)}</span>
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
    <div className="flex flex-col h-full min-h-0">
      <div className="flex items-center justify-between px-3 py-3 border-b shrink-0" style={{ borderColor: "var(--admiral-border)" }}>
        <span className="text-xs font-medium uppercase tracking-wider text-[var(--admiral-text-placeholder)]">
          任务列表
        </span>
        <Link
          to="/task/new"
          className="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium text-primary hover:bg-primary/10 rounded"
          style={{ borderRadius: "var(--admiral-radius-button)" }}
        >
          + 新建任务
        </Link>
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-1 min-h-0">
        {tasks.length === 0 ? (
          <p className="px-3 py-6 text-sm text-[var(--admiral-text-placeholder)]">暂无任务</p>
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
