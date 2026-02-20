// PRD 7.2: 执行时间线，规范 §6.6
// 连接线、节点样式（完成=绿实心，进行中=蓝脉冲，待执行=灰空心，失败=红实心）

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

function NodeDot({ status }: { status: TimelineStep["status"] }) {
  const isDone = status === "done";
  const isRunning = status === "running";
  const isFailed = status === "failed";
  const isPending = status === "pending";

  const bg =
    isDone
      ? "var(--color-success)"
      : isFailed
        ? "var(--color-danger)"
        : isRunning
          ? "var(--color-primary)"
          : "transparent";
  const border = isPending ? "2px solid #9CA3AF" : "none";

  return (
    <span
      className={isRunning ? "timeline-node-pulse" : ""}
      style={{
        width: 10,
        height: 10,
        borderRadius: "50%",
        flexShrink: 0,
        backgroundColor: bg,
        border,
        marginTop: 6,
      }}
      aria-hidden
    />
  );
}

export function TaskTimeline({ steps = [] }: TaskTimelineProps) {
  return (
    <div
      className="rounded-lg border p-4"
      style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-bg)", borderRadius: "var(--admiral-radius-card)" }}
    >
      <h4 className="text-sm font-medium text-[var(--admiral-text-primary)] mb-3">执行时间线</h4>
      {steps.length === 0 ? (
        <p className="text-sm text-[var(--admiral-text-placeholder)]">暂无执行记录</p>
      ) : (
        <ul className="space-y-0" style={{ listStyle: "none", padding: 0, margin: 0 }}>
          {steps.map((s, i) => (
            <li key={s.seq} className="flex items-start gap-3">
              {/* 左侧连接线 + 节点 */}
              <div className="flex flex-col items-center" style={{ width: 24 }}>
                <NodeDot status={s.status} />
                {i < steps.length - 1 && (
                  <div
                    style={{
                      width: 2,
                      flex: 1,
                      minHeight: 24,
                      backgroundColor: "var(--admiral-border)",
                    }}
                  />
                )}
              </div>
              <div className="flex-1 min-w-0 pb-4">
                <div className="flex items-baseline gap-2">
                  <span className="text-sm font-medium text-[var(--admiral-text-primary)]">
                    Step {s.seq}: {s.skill_id}
                  </span>
                  <span className="text-xs text-[var(--admiral-text-placeholder)]">
                    {s.started_at ? new Date(s.started_at).toLocaleTimeString() : "待执行"}
                  </span>
                </div>
                {s.params_summary && (
                  <span className="text-xs font-mono text-[var(--admiral-text-secondary)]">
                    {s.params_summary}
                  </span>
                )}
                {s.error && (
                  <p className="text-sm text-[var(--color-danger)] mt-0.5">{s.error}</p>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
