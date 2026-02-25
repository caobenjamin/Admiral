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
      className="rounded-lg border p-4"
      style={{
        borderColor: "var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        borderRadius: "var(--admiral-radius-card)",
        boxShadow: "var(--admiral-shadow-card)",
        position: "relative",
        paddingLeft: 16,
        borderLeft: "4px solid var(--color-success)",
      }}
    >
      <div className="mb-2">
        <span className="text-sm font-semibold text-[var(--admiral-text-primary)]">
          ✅ {isDryRun ? "Dry Run 通过" : "Test Run 通过"}
        </span>
      </div>
      <p className="text-sm text-[var(--admiral-text-secondary)] mb-3">
        {isDryRun
          ? "系统已完成预演，以下变更将被执行"
          : "测试运行已完成，可部署上线"}
      </p>
      <div style={{ display: "flex", justifyContent: "flex-end", gap: 12 }}>
        <button type="button" onClick={onCancel} disabled={disabled} className="btn btn-default">
          取消
        </button>
        <button
          type="button"
          onClick={onConfirm}
          disabled={disabled}
          className={isDryRun ? "btn btn-primary" : "btn btn-success"}
        >
          {isDryRun ? "开始 Test Run" : "部署上线"}
        </button>
      </div>
    </div>
  );
}
