import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useTaskStore, type TaskMeta, type TaskStatus } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";
import { Button } from "@/components/common/Button";

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
      className={`block w-full rounded-lg flex flex-col gap-1.5 transition-colors duration-150 shrink-0 ${
        selected
          ? "bg-[var(--admiral-task-selected-bg)]"
          : "border border-[var(--admiral-border)] bg-[var(--admiral-card)] hover:bg-[rgba(0,0,0,0.04)] hover:border-[var(--admiral-text-secondary)]"
      }`}
      style={{
        borderRadius: "var(--admiral-radius-card)",
        padding: "10px 12px",
        textAlign: "left",
        alignItems: "flex-start",
        ...(selected ? { borderLeft: "3px solid var(--color-primary)" } : {}),
      }}
    >
      <div
        className="flex items-center gap-2 w-full min-w-0"
        style={{ justifyContent: "flex-start", alignSelf: "stretch" }}
      >
        <span
          className={`h-2 w-2 shrink-0 rounded-full ${taskStatusDot(task.status)}`}
          style={{ borderRadius: "50%" }}
          aria-hidden
        />
        <span
          className="min-w-0 truncate"
          title={task.name}
          style={{
            flex: 1,
            fontSize: 14,
            fontWeight: selected ? 600 : 500,
            color: selected ? "var(--color-primary)" : "var(--admiral-text-primary)",
            textAlign: "left",
          }}
        >
          {task.name || "未命名任务"}
        </span>
      </div>
      <div
        className="flex items-center gap-1 pl-4 text-xs w-full"
        style={{
          color: "var(--admiral-text-secondary)",
          textAlign: "left",
          justifyContent: "flex-start",
        }}
      >
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
  const navigate = useNavigate();
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
      {/* 标题区 + 操作区 */}
      <div
        className="flex items-center justify-between shrink-0 border-b px-4 py-3"
        style={{
          borderColor: "var(--admiral-border)",
          backgroundColor: "var(--admiral-bg)",
        }}
      >
        <h3
          className="text-[var(--admiral-text-primary)]"
          style={{ fontSize: 14, fontWeight: 600, margin: 0 }}
        >
          任务列表
        </h3>
        <Button
          variant="primary"
          size="small"
          onClick={() => navigate("/task/new")}
          className="inline-flex items-center gap-1"
        >
          <span aria-hidden>+</span>
          <span>新建任务</span>
        </Button>
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
