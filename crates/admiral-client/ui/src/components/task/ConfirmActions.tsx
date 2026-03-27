interface ConfirmActionsProps {
  stage: "plan_ready";
  onConfirm: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

export function ConfirmActions({ stage: _stage, onConfirm, onCancel, disabled }: ConfirmActionsProps) {
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
          <span className="admiral-chip admiral-chip-primary">计划就绪</span>
          <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>可以开始执行</span>
        </div>
        <p style={{ margin: 0, fontSize: 13, color: "var(--admiral-text-secondary)" }}>
          执行计划已生成，确认后开始运行
        </p>
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, flexShrink: 0 }}>
        <button type="button" onClick={onCancel} disabled={disabled} className="btn btn-default">
          暂不执行
        </button>
        <button type="button" onClick={onConfirm} disabled={disabled} className="btn btn-primary">
          开始执行
        </button>
      </div>
    </div>
  );
}
