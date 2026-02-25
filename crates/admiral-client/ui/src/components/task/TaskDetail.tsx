import { useNavigate } from "react-router-dom";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, type TaskStatus } from "@/stores/taskStore";
import { useTasks } from "@/hooks/useTasks";
import { TaskTimeline } from "./TaskTimeline";
import { ConfirmActions } from "./ConfirmActions";
import { Button } from "@/components/common/Button";

function statusLabel(s: TaskStatus): string {
  const map: Record<string, string> = {
    draft: "草稿",
    planning: "规划中",
    plan_ready: "计划就绪",
    dry_running: "Dry Run 中",
    dry_run_passed: "Dry Run 通过",
    test_running: "测试运行中",
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

function typeLabel(t: "instant" | "scheduled" | "heartbeat"): string {
  if (t === "instant") return "即时任务";
  if (t === "heartbeat") return "心跳任务";
  return "定时任务";
}

export function TaskDetail() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const task = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : null;
  const navigate = useNavigate();
  const { runTestRun, runProduction, refresh } = useTasks();

  if (!task) {
    return (
      <div
        className="rounded-lg border p-8 flex flex-col items-center justify-center text-center min-h-[200px]"
        style={{
          borderColor: "var(--admiral-border)",
          backgroundColor: "var(--admiral-card)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "var(--admiral-shadow-card)",
        }}
      >
        <span className="text-2xl mb-4" aria-hidden>📋</span>
        <p className="text-sm text-[var(--admiral-text-secondary)] mb-4">
          在左侧选择任务，或创建新任务
        </p>
        <Button variant="primary" onClick={() => navigate("/task/new")} className="inline-flex items-center gap-2">
          <span aria-hidden>+</span>
          <span>新建任务</span>
        </Button>
      </div>
    );
  }

  const showProgress = ["running", "dry_running", "test_running", "succeeded", "failed"].includes(task.status);
  const progressPercent =
    task.status === "succeeded"
      ? 100
      : task.status === "failed"
        ? 60
        : ["running", "dry_running", "test_running"].includes(task.status)
          ? 60
          : 0;
  const showConfirmActions =
    task.status === "dry_run_passed" || task.status === "test_run_passed";

  const handleConfirm = async () => {
    try {
      if (task.status === "dry_run_passed") await runTestRun(task.id);
      else if (task.status === "test_run_passed") await runProduction(task.id);
      await refresh();
    } catch {
      await refresh();
    }
  };
  const handleCancel = async () => {
    await refresh();
  };

  return (
    <div
      className="rounded-lg border p-4"
      style={{
        borderColor: "var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        borderRadius: "var(--admiral-radius-card)",
        boxShadow: "var(--admiral-shadow-card)",
      }}
    >
      <div style={{ marginBottom: 16 }}>
        <h2 className="text-base font-semibold text-[var(--admiral-text-primary)]" style={{ fontSize: 16, fontWeight: 600, margin: 0 }}>
          {task.name || "未命名任务"}
        </h2>
        <p className="text-sm text-[var(--admiral-text-secondary)] mt-0.5">
          {typeLabel(task.type)} · {statusLabel(task.status)}
        </p>
      </div>
      {showProgress && progressPercent > 0 && (
        <div style={{ marginBottom: 16 }}>
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
      {/* 执行时间线区块 */}
      <div style={{ marginBottom: 16 }}>
        <h3
          className="text-[var(--admiral-text-primary)] mb-2"
          style={{ fontSize: 14, fontWeight: 600, margin: "0 0 8px 0" }}
        >
          执行时间线
        </h3>
        <TaskTimeline hideTitle />
      </div>
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
