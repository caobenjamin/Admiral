import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, type TaskStatus } from "@/stores/taskStore";
import { useTasks } from "@/hooks/useTasks";
import { Button } from "@/components/common/Button";
import type { PendingApproval, CompletenessLevel } from "@/lib/types";
import * as tauri from "@/lib/tauri";

function statusLabel(s: TaskStatus): string {
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

function completenessLabel(c: CompletenessLevel): string {
  const map: Record<string, string> = {
    incomplete: "不完整",
    partial: "部分完整",
    sufficient: "基本完整",
    complete: "完整",
  };
  return map[c] ?? c;
}

function completenessPercent(c: CompletenessLevel): number {
  const map: Record<string, number> = { incomplete: 15, partial: 40, sufficient: 70, complete: 100 };
  return map[c] ?? 0;
}

function completenessColor(c: CompletenessLevel): string {
  if (c === "complete") return "var(--color-success)";
  if (c === "sufficient") return "var(--color-primary)";
  if (c === "partial") return "var(--color-warning, #f0ad4e)";
  return "var(--admiral-text-placeholder)";
}

function typeLabel(t: string): string {
  if (t === "instant") return "手动触发";
  if (t === "heartbeat") return "心跳任务";
  return "定时任务";
}

function formatDate(v: string): string {
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return "-";
  return d.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

type BtnVariant = "default" | "primary" | "success" | "warning" | "danger";

function getAvailableActions(status: TaskStatus, taskType: string): { label: string; targetStatus: TaskStatus; variant: BtnVariant }[] {
  const actions: { label: string; targetStatus: TaskStatus; variant: BtnVariant }[] = [];
  switch (status) {
    case "configuring":
      actions.push({ label: "开始试运行", targetStatus: "testing", variant: "primary" });
      break;
    case "testing":
      actions.push({ label: "退回配置", targetStatus: "configuring", variant: "default" });
      break;
    case "ready":
      if (taskType === "instant") {
        actions.push({ label: "执行", targetStatus: "running", variant: "primary" });
      } else {
        actions.push({ label: "发布", targetStatus: "active", variant: "primary" });
        actions.push({ label: "立即执行", targetStatus: "running", variant: "default" });
      }
      actions.push({ label: "调整配置", targetStatus: "configuring", variant: "default" });
      break;
    case "active":
      actions.push({ label: "暂停", targetStatus: "paused", variant: "warning" });
      actions.push({ label: "调整配置", targetStatus: "configuring", variant: "default" });
      break;
    case "running":
      actions.push({ label: "暂停", targetStatus: "paused", variant: "warning" });
      break;
    case "paused":
      actions.push({ label: "继续", targetStatus: "running", variant: "primary" });
      if (taskType !== "instant") actions.push({ label: "恢复调度", targetStatus: "active", variant: "default" });
      break;
    case "failed":
      actions.push({ label: "重新配置", targetStatus: "configuring", variant: "primary" });
      break;
    case "completed":
      if (taskType !== "instant") actions.push({ label: "重新调度", targetStatus: "active", variant: "default" });
      actions.push({ label: "调整配置", targetStatus: "configuring", variant: "default" });
      break;
    case "cancelled":
      actions.push({ label: "重新开始", targetStatus: "draft", variant: "default" });
      break;
  }
  return actions;
}

function ApprovalPanel({ taskId, approval, onDone }: { taskId: string; approval: PendingApproval; onDone: () => void }) {
  const { approveOperation, rejectOperation } = useTasks();
  const [loading, setLoading] = useState(false);

  const handleApprove = async (autoFuture: boolean) => {
    setLoading(true);
    try { await approveOperation(taskId, autoFuture); } finally { setLoading(false); onDone(); }
  };
  const handleReject = async () => {
    setLoading(true);
    try { await rejectOperation(taskId); } finally { setLoading(false); onDone(); }
  };

  return (
    <div style={{ border: "1px solid var(--color-warning, #f0ad4e)", borderRadius: 10, padding: 14, marginBottom: 12, background: "rgba(240, 173, 78, 0.06)" }}>
      <h4 style={{ margin: "0 0 8px", fontSize: 14, fontWeight: 700 }}>操作确认</h4>
      <p style={{ fontSize: 13, margin: "0 0 4px", color: "var(--admiral-text-secondary)" }}>
        即将执行写操作 <strong>{approval.skill_id}</strong>（步骤 {approval.step_seq}）
      </p>
      <p style={{ fontSize: 12, margin: "0 0 10px", color: "var(--admiral-text-placeholder)", wordBreak: "break-all" }}>
        {approval.params_summary.length > 200 ? approval.params_summary.slice(0, 200) + "..." : approval.params_summary}
      </p>
      <div style={{ display: "flex", gap: 8 }}>
        <Button variant="primary" size="small" disabled={loading} onClick={() => handleApprove(false)}>仅此次批准</Button>
        <Button variant="default" size="small" disabled={loading} onClick={() => handleApprove(true)}>以后不再询问</Button>
        <Button variant="danger" size="small" disabled={loading} onClick={handleReject}>拒绝</Button>
      </div>
    </div>
  );
}

function SpecViewer({ taskId }: { taskId: string }) {
  const [spec, setSpec] = useState<string>("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    tauri.taskLoadSpec(taskId).then(setSpec).catch(() => setSpec("")).finally(() => setLoading(false));
  }, [taskId]);

  if (loading) return <p style={{ fontSize: 13, color: "var(--admiral-text-placeholder)", padding: 12 }}>加载中...</p>;
  if (!spec) return <p style={{ fontSize: 13, color: "var(--admiral-text-placeholder)", padding: 12 }}>尚无任务规格文档，请在对话中描述你的需求。</p>;

  return (
    <div style={{ padding: 12, fontSize: 13, lineHeight: 1.7, color: "var(--admiral-text-primary)", whiteSpace: "pre-wrap", fontFamily: "inherit" }}>
      {spec}
    </div>
  );
}

function RunHistory({ taskId }: { taskId: string }) {
  const [reports, setReports] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    tauri.taskListRunReports(taskId).then(setReports).catch(() => setReports([])).finally(() => setLoading(false));
  }, [taskId]);

  if (loading) return <p style={{ fontSize: 13, color: "var(--admiral-text-placeholder)", padding: 12 }}>加载中...</p>;
  if (reports.length === 0) return <p style={{ fontSize: 13, color: "var(--admiral-text-placeholder)", padding: 12 }}>暂无运行记录。</p>;

  return (
    <div style={{ padding: 12, fontSize: 13, color: "var(--admiral-text-secondary)" }}>
      <p style={{ margin: "0 0 8px", fontWeight: 600 }}>运行报告列表</p>
      {reports.map((r) => (
        <div key={r} style={{ padding: "4px 0", borderBottom: "1px solid var(--admiral-border)" }}>
          {r.replace(".md", "")}
        </div>
      ))}
    </div>
  );
}

export function TaskDetail() {
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const { fetchTasks } = useTaskStore();
  const task = selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) : null;
  const navigate = useNavigate();
  const { getPendingApproval } = useTasks();

  const [pendingApproval, setPendingApproval] = useState<PendingApproval | null>(null);
  const [actionLoading, setActionLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<"spec" | "history">("spec");

  useEffect(() => {
    if (task?.status === "waiting_for_approval") {
      getPendingApproval(task.id).then(setPendingApproval).catch(() => setPendingApproval(null));
    } else {
      setPendingApproval(null);
    }
  }, [task?.id, task?.status, getPendingApproval]);

  const handleActionDone = useCallback(async () => {
    setPendingApproval(null);
    await fetchTasks();
  }, [fetchTasks]);

  const handleTransition = useCallback(
    async (targetStatus: string) => {
      if (!task) return;
      setActionLoading(true);
      try {
        await tauri.taskTransition(task.id, targetStatus as TaskStatus);
        await fetchTasks();
      } catch (e) {
        console.error("transition failed:", e);
      } finally {
        setActionLoading(false);
      }
    },
    [task, fetchTasks]
  );

  if (!task) {
    return (
      <div className="admiral-surface" style={{ padding: 28, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", textAlign: "center", minHeight: 220 }}>
        <p style={{ margin: 0, fontSize: 14, color: "var(--admiral-text-secondary)" }}>在左侧选择任务，或创建新任务</p>
        <Button variant="primary" onClick={() => navigate("/task/new")} style={{ marginTop: 14 }}>
          <span>+ 新建任务</span>
        </Button>
      </div>
    );
  }

  const taskType = task.type === "scheduled" ? "cron" : task.type;
  const actions = getAvailableActions(task.status, taskType);
  const cPercent = completenessPercent(task.completeness);

  return (
    <div className="admiral-surface" style={{ padding: 14 }}>
      {/* Header */}
      <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, background: "linear-gradient(180deg, #fbfeff 0%, #f4f9ff 100%)", padding: 12, marginBottom: 12 }}>
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12, marginBottom: 8 }}>
          <div style={{ minWidth: 0 }}>
            <h2 style={{ fontSize: 17, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>{task.name || "未命名任务"}</h2>
            <p style={{ fontSize: 12, margin: "4px 0 0", color: "var(--admiral-text-secondary)" }}>
              {typeLabel(task.type)} · 更新于 {formatDate(task.updated_at)}
            </p>
          </div>
          <span className={statusClass(task.status)}>{statusLabel(task.status)}</span>
        </div>

        {/* Completeness bar */}
        <div style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
            <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>信息完整度</span>
            <span style={{ fontSize: 12, color: completenessColor(task.completeness) }}>{completenessLabel(task.completeness)}</span>
          </div>
          <div style={{ height: 6, borderRadius: 4, backgroundColor: "var(--admiral-border)", overflow: "hidden" }}>
            <div style={{ width: `${cPercent}%`, height: "100%", borderRadius: 4, transition: "width 0.3s ease", backgroundColor: completenessColor(task.completeness) }} />
          </div>
        </div>

        {/* Actions */}
        {actions.length > 0 && (
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            {actions.map((a) => (
              <Button key={a.targetStatus} variant={a.variant} size="small" disabled={actionLoading} onClick={() => handleTransition(a.targetStatus)}>
                {a.label}
              </Button>
            ))}
          </div>
        )}
      </div>

      {/* Approval panel */}
      {task.status === "waiting_for_approval" && pendingApproval && (
        <ApprovalPanel taskId={task.id} approval={pendingApproval} onDone={handleActionDone} />
      )}

      {/* Tabs: Spec / History */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid var(--admiral-border)", marginBottom: 8 }}>
        {(["spec", "history"] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
            style={{
              padding: "8px 16px",
              fontSize: 13,
              fontWeight: activeTab === tab ? 600 : 400,
              color: activeTab === tab ? "var(--color-primary)" : "var(--admiral-text-secondary)",
              borderBottom: activeTab === tab ? "2px solid var(--color-primary)" : "2px solid transparent",
              background: "none",
              border: "none",
              borderBottomWidth: 2,
              borderBottomStyle: "solid",
              cursor: "pointer",
            }}
          >
            {tab === "spec" ? "任务规格" : "运行历史"}
          </button>
        ))}
      </div>

      <div style={{ minHeight: 100, maxHeight: 280, overflowY: "auto", border: "1px solid var(--admiral-border)", borderRadius: 8 }}>
        {activeTab === "spec" ? <SpecViewer taskId={task.id} /> : <RunHistory taskId={task.id} />}
      </div>
    </div>
  );
}
