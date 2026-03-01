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

function statusClass(s: TaskStatus): string {
  if (s === "running" || s === "dry_running" || s === "test_running") return "admiral-chip admiral-chip-warning";
  if (s === "succeeded" || s === "dry_run_passed" || s === "test_run_passed") return "admiral-chip admiral-chip-success";
  if (s === "failed") return "admiral-chip admiral-chip-danger";
  if (s === "scheduled" || s === "deployed") return "admiral-chip admiral-chip-primary";
  return "admiral-chip";
}

function typeLabel(t: "instant" | "scheduled" | "heartbeat"): string {
  if (t === "instant") return "即时任务";
  if (t === "heartbeat") return "心跳任务";
  return "定时任务";
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

export function TaskDetail() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const task = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : null;
  const navigate = useNavigate();
  const { runTestRun, runProduction, refresh } = useTasks();

  if (!task) {
    return (
      <div
        className="admiral-surface"
        style={{ padding: 28, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", minHeight: 220 }}
      >
        <span style={{ fontSize: 30, marginBottom: 10 }} aria-hidden>
          📋
        </span>
        <p style={{ margin: 0, fontSize: 14, color: "var(--admiral-text-secondary)" }}>在左侧选择任务，或创建新任务</p>
        <Button variant="primary" onClick={() => navigate("/task/new")} className="inline-flex items-center gap-2" style={{ marginTop: 14 }}>
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

  const showConfirmActions = task.status === "dry_run_passed" || task.status === "test_run_passed";

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
    <div className="admiral-surface" style={{ padding: 14 }}>
      <div
        style={{
          border: "1px solid var(--admiral-border)",
          borderRadius: 12,
          background: "linear-gradient(180deg, #fbfeff 0%, #f4f9ff 100%)",
          padding: 12,
          marginBottom: 12,
        }}
      >
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12, marginBottom: 8 }}>
          <div style={{ minWidth: 0 }}>
            <h2 style={{ fontSize: 17, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>{task.name || "未命名任务"}</h2>
            <p style={{ fontSize: 12, margin: "4px 0 0", color: "var(--admiral-text-secondary)" }}>
              {typeLabel(task.type)} · 更新于 {formatDate(task.updated_at)}
            </p>
          </div>
          <span className={statusClass(task.status)}>{statusLabel(task.status)}</span>
        </div>

        {showProgress && progressPercent > 0 && (
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
              <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>执行进度</span>
              <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>{progressPercent}%</span>
            </div>
            <div
              style={{ height: 8, borderRadius: 6, backgroundColor: "var(--admiral-border)", overflow: "hidden" }}
              role="progressbar"
              aria-valuenow={progressPercent}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <div
                className={task.status === "running" ? "task-progress-animated" : ""}
                style={{
                  width: `${progressPercent}%`,
                  height: "100%",
                  borderRadius: 6,
                  transition: "width 0.24s ease",
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
      </div>

      <div style={{ marginBottom: 12 }}>
        <h3 style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)", margin: "0 0 8px" }}>执行时间线</h3>
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
