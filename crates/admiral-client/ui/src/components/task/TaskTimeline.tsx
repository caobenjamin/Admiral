// PRD 7.2: 执行时间线，数据结构预留接 task_runs / step_results

export interface TimelineStep {
  seq: number;
  skill_id: string;
  params_summary?: string;
  started_at?: string;
  finished_at?: string;
  status: "pending" | "running" | "done" | "failed";
  error?: string;
}

interface TaskTimelineProps {
  steps?: TimelineStep[];
}

export function TaskTimeline({ steps = [] }: TaskTimelineProps) {
  if (steps.length === 0) {
    return (
      <div
        className="rounded-lg border p-4"
        style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-bg)", borderRadius: "var(--admiral-radius-card)" }}
      >
        <h4 className="text-sm font-medium text-[var(--admiral-text-primary)] mb-2">执行时间线</h4>
        <p className="text-sm text-[var(--admiral-text-placeholder)]">暂无执行记录</p>
      </div>
    );
  }

  return (
    <div
      className="rounded-lg border p-4"
      style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-bg)", borderRadius: "var(--admiral-radius-card)" }}
    >
      <h4 className="text-sm font-medium text-[var(--admiral-text-primary)] mb-3">执行时间线</h4>
      <ul className="space-y-2">
        {steps.map((s) => (
          <li key={s.seq} className="flex items-start gap-3 text-sm">
            <span className="text-[var(--admiral-text-placeholder)] shrink-0 w-12 text-xs">
              {s.started_at ? new Date(s.started_at).toLocaleTimeString() : "—"}
            </span>
            <span
              className={`w-2 h-2 mt-1.5 rounded-full shrink-0 ${
                s.status === "done"
                  ? "bg-success"
                  : s.status === "running"
                    ? "bg-warning"
                    : s.status === "failed"
                      ? "bg-danger"
                      : "bg-gray-300"
              }`}
            />
            <div className="min-w-0 flex-1">
              <span className="text-[var(--admiral-text-primary)]">{s.skill_id}</span>
              {s.params_summary && (
                <span className="text-[var(--admiral-text-secondary)] ml-2">({s.params_summary})</span>
              )}
              {s.error && (
                <p className="text-danger text-xs mt-0.5">{s.error}</p>
              )}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
