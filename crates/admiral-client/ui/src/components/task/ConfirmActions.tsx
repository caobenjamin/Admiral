interface ConfirmActionsProps {
  stage: "dry_run_passed" | "test_run_passed";
  onConfirm: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

export function ConfirmActions({ stage, onConfirm, onCancel, disabled }: ConfirmActionsProps) {
  const isDryRun = stage === "dry_run_passed";

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
        borderLeft: "4px solid var(--color-success)",
      }}
    >
      <div style={{ minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 2 }}>
          <span className="admiral-chip admiral-chip-success">{isDryRun ? "Dry Run 通过" : "Test Run 通过"}</span>
          <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>下一步可执行</span>
        </div>
        <p style={{ margin: 0, fontSize: 13, color: "var(--admiral-text-secondary)" }}>
          {isDryRun ? "系统已完成预演，建议执行 Test Run" : "测试运行已完成，可部署上线"}
        </p>
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, flexShrink: 0 }}>
        <button type="button" onClick={onCancel} disabled={disabled} className="btn btn-default">
          暂不执行
        </button>
        <button type="button" onClick={onConfirm} disabled={disabled} className={isDryRun ? "btn btn-primary" : "btn btn-success"}>
          {isDryRun ? "开始 Test Run" : "部署上线"}
        </button>
      </div>
    </div>
  );
}
