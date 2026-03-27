import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, type TaskStatus } from "@/stores/taskStore";
import { useTasks } from "@/hooks/useTasks";
import { TaskTimeline } from "./TaskTimeline";
import { Button } from "@/components/common/Button";
import type { PendingApproval } from "@/lib/types";

function statusLabel(s: TaskStatus): string {
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
  if (s === "waiting_for_approval") return "admiral-chip admiral-chip-warning";
  if (s === "partially_succeeded") return "admiral-chip admiral-chip-warning";
  if (s === "scheduled" || s === "plan_ready") return "admiral-chip admiral-chip-primary";
  return "admiral-chip";
}

function typeLabel(t: string): string {
  if (t === "instant") return "即时任务";
  if (t === "heartbeat") return "心跳任务";
  return "定时任务";
}

function categoryLabel(c: string): string {
  const map: Record<string, string> = {
    data_fetch: "数据拉取",
    create: "创建类",
    update: "更新类",
    mixed: "混合型",
  };
  return map[c] ?? c;
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

function ApprovalPanel({
  taskId,
  approval,
  onDone,
}: {
  taskId: string;
  approval: PendingApproval;
  onDone: () => void;
}) {
  const { approveOperation, rejectOperation } = useTasks();
  const [loading, setLoading] = useState(false);

  const handleApprove = async (autoFuture: boolean) => {
    setLoading(true);
    try {
      await approveOperation(taskId, autoFuture);
    } finally {
      setLoading(false);
      onDone();
    }
  };

  const handleReject = async () => {
    setLoading(true);
    try {
      await rejectOperation(taskId);
    } finally {
      setLoading(false);
      onDone();
    }
  };

  return (
    <div
      style={{
        border: "1px solid var(--color-warning, #f0ad4e)",
        borderRadius: 10,
        padding: 14,
        marginBottom: 12,
        background: "rgba(240, 173, 78, 0.06)",
      }}
    >
      <h4 style={{ margin: "0 0 8px", fontSize: 14, fontWeight: 700 }}>操作确认</h4>
      <p style={{ fontSize: 13, margin: "0 0 4px", color: "var(--admiral-text-secondary)" }}>
        即将执行写操作 <strong>{approval.skill_id}</strong>（步骤 {approval.step_seq}）
      </p>
      <p style={{ fontSize: 12, margin: "0 0 10px", color: "var(--admiral-text-placeholder)", wordBreak: "break-all" }}>
        {approval.params_summary.length > 200
          ? approval.params_summary.slice(0, 200) + "..."
          : approval.params_summary}
      </p>
      <div style={{ display: "flex", gap: 8 }}>
        <Button variant="primary" size="sm" disabled={loading} onClick={() => handleApprove(false)}>
          仅此次批准
        </Button>
        <Button variant="secondary" size="sm" disabled={loading} onClick={() => handleApprove(true)}>
          以后不再询问
        </Button>
        <Button variant="danger" size="sm" disabled={loading} onClick={handleReject}>
          拒绝
        </Button>
      </div>
    </div>
  );
}

function PartialSuccessPanel({
  taskId,
  taskCategory,
  resumeFromStep,
  completedSteps,
  onDone,
}: {
  taskId: string;
  taskCategory: string;
  resumeFromStep?: number | null;
  completedSteps: number[];
  onDone: () => void;
}) {
  const { resumeTask } = useTasks();
  const [loading, setLoading] = useState(false);

  const defaultResume = taskCategory === "data_fetch" || taskCategory === "update";

  const handleResume = async () => {
    setLoading(true);
    try {
      await resumeTask(taskId, resumeFromStep, "production");
    } finally {
      setLoading(false);
      onDone();
    }
  };

  const handleFullRerun = async () => {
    setLoading(true);
    try {
      await resumeTask(taskId, 0, "production");
    } finally {
      setLoading(false);
      onDone();
    }
  };

  return (
    <div
      style={{
        border: "1px solid var(--color-warning, #f0ad4e)",
        borderRadius: 10,
        padding: 14,
        marginBottom: 12,
        background: "rgba(240, 173, 78, 0.06)",
      }}
    >
      <h4 style={{ margin: "0 0 8px", fontSize: 14, fontWeight: 700 }}>部分完成</h4>
      <p style={{ fontSize: 13, margin: "0 0 4px", color: "var(--admiral-text-secondary)" }}>
        已完成 {completedSteps.length} 个步骤，在步骤 {resumeFromStep ?? "?"} 处中断。
      </p>
      <p style={{ fontSize: 12, margin: "0 0 10px", color: "var(--admiral-text-placeholder)" }}>
        任务类型：{categoryLabel(taskCategory)}
        {defaultResume
          ? "（建议从断点续跑）"
          : taskCategory === "create"
            ? "（建议全量重跑）"
            : "（请选择）"}
      </p>
      <div style={{ display: "flex", gap: 8 }}>
        <Button
          variant={defaultResume ? "primary" : "secondary"}
          size="sm"
          disabled={loading}
          onClick={handleResume}
        >
          从断点续跑
        </Button>
        <Button
          variant={!defaultResume ? "primary" : "secondary"}
          size="sm"
          disabled={loading}
          onClick={handleFullRerun}
        >
          全量重跑
        </Button>
      </div>
    </div>
  );
}

export function TaskDetail() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const task = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : null;
  const navigate = useNavigate();
  const { getPendingApproval, refresh } = useTasks();

  const [pendingApproval, setPendingApproval] = useState<PendingApproval | null>(null);

  useEffect(() => {
    if (task?.status === "waiting_for_approval") {
      getPendingApproval(task.id).then(setPendingApproval).catch(() => setPendingApproval(null));
    } else {
      setPendingApproval(null);
    }
  }, [task?.id, task?.status, getPendingApproval]);

  const handleActionDone = async () => {
    setPendingApproval(null);
    await refresh();
  };

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

  const showProgress = ["running", "succeeded", "failed", "partially_succeeded"].includes(task.status);
  const progressPercent =
    task.status === "succeeded"
      ? 100
      : task.status === "failed"
        ? 60
        : task.status === "partially_succeeded"
          ? 50
          : task.status === "running"
            ? 60
            : 0;

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
              {typeLabel(task.type)} · {categoryLabel(task.task_category)} · 更新于 {formatDate(task.updated_at)}
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
                        : task.status === "partially_succeeded"
                          ? "var(--color-warning, #f0ad4e)"
                          : "var(--color-primary)",
                }}
              />
            </div>
          </div>
        )}
      </div>

      {/* Approval panel */}
      {task.status === "waiting_for_approval" && pendingApproval && (
        <ApprovalPanel
          taskId={task.id}
          approval={pendingApproval}
          onDone={handleActionDone}
        />
      )}

      {/* Partial success recovery panel */}
      {task.status === "partially_succeeded" && (
        <PartialSuccessPanel
          taskId={task.id}
          taskCategory={task.task_category}
          resumeFromStep={undefined}
          completedSteps={[]}
          onDone={handleActionDone}
        />
      )}

      <div style={{ marginBottom: 12 }}>
        <h3 style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)", margin: "0 0 8px" }}>执行时间线</h3>
        <TaskTimeline hideTitle />
      </div>
    </div>
  );
}
