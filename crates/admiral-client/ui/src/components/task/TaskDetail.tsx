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
        className="rounded-lg border p-6 text-center"
        style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", borderRadius: "var(--admiral-radius-card)" }}
      >
        <p className="text-sm text-[var(--admiral-text-placeholder)]">在左侧选择任务，或点击「新建任务」创建</p>
      </div>
    );
  }

  const progressPercent = task.status === "succeeded" ? 100 : task.status === "running" ? 60 : 0;
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
        <h3 className="text-base font-semibold text-[var(--admiral-text-primary)]">{task.name || "未命名任务"}</h3>
        <p className="text-sm text-[var(--admiral-text-secondary)] mt-0.5">
          类型: {typeLabel(task.type)} · 状态: {statusLabel(task.status)}
        </p>
      </div>
      {progressPercent > 0 && (
        <div>
          <div className="flex justify-between text-xs text-[var(--admiral-text-secondary)] mb-1">
            <span>进度</span>
            <span>{progressPercent}%</span>
          </div>
          <div className="h-2 rounded-full overflow-hidden bg-[var(--admiral-border)]">
            <div
              className="h-full bg-primary rounded-full transition-all duration-300"
              style={{ width: `${progressPercent}%` }}
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
