interface ConfirmActionsProps {
  stage: "ready" | "configuring";
  onConfirm: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

export function ConfirmActions({ stage, onConfirm, onCancel, disabled }: ConfirmActionsProps) {
  const labelMap: Record<string, { chip: string; desc: string; btn: string }> = {
    ready: { chip: "就绪", desc: "任务配置已完成，可以开始执行", btn: "开始执行" },
    configuring: { chip: "配置中", desc: "信息基本完整，建议进行试运行验证", btn: "开始试运行" },
  };
  const labels = labelMap[stage] ?? labelMap.ready;

  return (
    <div
      className="admiral-surface-raised"
      style={{
        position: "relative",
        padding: 12,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 12,
        borderLeft: "4px solid var(--color-primary)",
      }}
    >
      <div style={{ minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 2 }}>
          <span className="admiral-chip admiral-chip-primary">{labels.chip}</span>
          <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>可以开始</span>
        </div>
        <p style={{ margin: 0, fontSize: 13, color: "var(--admiral-text-secondary)" }}>
          {labels.desc}
        </p>
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, flexShrink: 0 }}>
        <button type="button" onClick={onCancel} disabled={disabled} className="btn btn-default">
          暂不执行
        </button>
        <button type="button" onClick={onConfirm} disabled={disabled} className="btn btn-primary">
          {labels.btn}
        </button>
      </div>
    </div>
  );
}
