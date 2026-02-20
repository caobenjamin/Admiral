import { Link } from "react-router-dom";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, type TaskStatus } from "@/stores/taskStore";
import { TaskTimeline } from "./TaskTimeline";
import { ConfirmActions } from "./ConfirmActions";

function statusLabel(s: TaskStatus): string {
  const map: Record<TaskStatus, string> = {
    draft: "草稿",
    dry_running: "Dry Run 中",
    dry_run_passed: "Dry Run 通过",
    test_run_ready: "待测试",
    test_run_running: "测试运行中",
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

function typeLabel(t: "instant" | "scheduled"): string {
  return t === "instant" ? "即时任务" : "定时任务";
}

export function TaskDetail() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const task = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : null;

  if (!task) {
    return (
      <div
        className="rounded-lg border p-8 flex flex-col items-center justify-center text-center min-h-[200px]"
        style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", borderRadius: "var(--admiral-radius-card)" }}
      >
        <span className="text-2xl mb-4" aria-hidden>📋</span>
        <p className="text-sm text-[var(--admiral-text-secondary)] mb-4">
          在左侧选择任务，或创建新任务
        </p>
        <Link
          to="/task/new"
          className="inline-flex items-center gap-2 px-4 py-2.5 text-sm font-medium text-white rounded-md bg-[var(--color-primary)] hover:opacity-90 transition-opacity"
          style={{ borderRadius: "var(--admiral-radius-button)" }}
        >
          <span>+</span>
          <span>新建任务</span>
        </Link>
      </div>
    );
  }

  const showProgress = ["running", "dry_running", "test_run_running", "succeeded", "failed"].includes(task.status);
  const progressPercent =
    task.status === "succeeded"
      ? 100
      : task.status === "failed"
        ? 60
        : ["running", "dry_running", "test_run_running"].includes(task.status)
          ? 60
          : 0;
  const showConfirmActions =
    task.status === "dry_run_passed" || task.status === "test_run_passed";

  const handleConfirm = () => {
    // 占位：Phase 5 接任务引擎
  };
  const handleCancel = () => {
    // 占位：Phase 5 接任务引擎
  };

  return (
    <div
      className="rounded-lg border p-4 space-y-4"
      style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", borderRadius: "var(--admiral-radius-card)" }}
    >
      <div>
        <h2 className="text-base font-semibold text-[var(--admiral-text-primary)]" style={{ fontSize: 16, fontWeight: 600 }}>
          {task.name || "未命名任务"}
        </h2>
        <p className="text-sm text-[var(--admiral-text-secondary)] mt-0.5">
          {typeLabel(task.type)} · {statusLabel(task.status)}
        </p>
      </div>
      {showProgress && progressPercent > 0 && (
        <div>
          <div className="flex justify-between items-center gap-2 mb-1">
            <span className="text-xs text-[var(--admiral-text-secondary)]">进度</span>
            <span className="text-xs text-[var(--admiral-text-placeholder)]">{progressPercent}%</span>
          </div>
          <div
            className="overflow-hidden rounded"
            style={{ height: 8, borderRadius: 4, backgroundColor: "var(--admiral-border)" }}
            role="progressbar"
            aria-valuenow={progressPercent}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div
              className={`h-full rounded transition-all duration-300 ${task.status === "running" ? "task-progress-animated" : ""}`}
              style={{
                width: `${progressPercent}%`,
                borderRadius: 4,
                backgroundColor:
                  task.status === "failed"
                    ? "var(--color-danger)"
                    : task.status === "succeeded"
                      ? "var(--color-success)"
                      : "var(--color-primary)",
              }}
            />
          </div>
        </div>
      )}
      <TaskTimeline />
      {showConfirmActions && (
        <ConfirmActions
          stage={task.status === "dry_run_passed" ? "dry_run_passed" : "test_run_passed"}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
    </div>
  );
}
