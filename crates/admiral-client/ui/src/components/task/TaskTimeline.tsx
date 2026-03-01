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
  hideTitle?: boolean;
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
  const border = isPending ? "2px solid #9fb3c8" : "none";

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
        marginTop: 7,
      }}
      aria-hidden
    />
  );
}

export function TaskTimeline({ steps = [], hideTitle = false }: TaskTimelineProps) {
  return (
    <div
      style={{
        border: "1px solid var(--admiral-border)",
        borderRadius: 12,
        background: "var(--admiral-card-muted)",
        padding: 12,
      }}
    >
      {!hideTitle && (
        <h4 style={{ fontSize: 13, fontWeight: 700, color: "var(--admiral-text-primary)", margin: "0 0 10px" }}>
          执行时间线
        </h4>
      )}

      {steps.length === 0 ? (
        <p style={{ fontSize: 13, color: "var(--admiral-text-placeholder)", margin: 0 }}>暂无执行记录</p>
      ) : (
        <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
          {steps.map((s, i) => (
            <li key={s.seq} style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
              <div style={{ display: "flex", flexDirection: "column", alignItems: "center", width: 20 }}>
                <NodeDot status={s.status} />
                {i < steps.length - 1 && (
                  <div
                    style={{ width: 2, flex: 1, minHeight: 26, backgroundColor: "var(--admiral-border)" }}
                  />
                )}
              </div>

              <div style={{ flex: 1, minWidth: 0, paddingBottom: 10 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                  <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>
                    Step {s.seq}: {s.skill_id}
                  </span>
                  <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>
                    {s.started_at ? new Date(s.started_at).toLocaleTimeString() : "待执行"}
                  </span>
                </div>

                {s.params_summary && (
                  <span
                    style={{
                      fontFamily: "var(--admiral-font-mono)",
                      fontSize: 12,
                      color: "var(--admiral-text-secondary)",
                    }}
                  >
                    {s.params_summary}
                  </span>
                )}

                {s.error && <p style={{ color: "var(--color-danger)", margin: "4px 0 0", fontSize: 13 }}>{s.error}</p>}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
