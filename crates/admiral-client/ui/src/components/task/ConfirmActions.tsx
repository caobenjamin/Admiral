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
        <button
          type="button"
          onClick={onCancel}
          disabled={disabled}
          style={{
            padding: "8px 16px",
            fontSize: 14,
            fontWeight: 500,
            borderRadius: "var(--admiral-radius-button)",
            border: "1px solid var(--admiral-border)",
            backgroundColor: "var(--admiral-card)",
            color: "var(--admiral-text-primary)",
            cursor: disabled ? "not-allowed" : "pointer",
            opacity: disabled ? 0.5 : 1,
          }}
        >
          取消
        </button>
        <button
          type="button"
          onClick={onConfirm}
          disabled={disabled}
          style={{
            padding: "8px 16px",
            fontSize: 14,
            fontWeight: 500,
            borderRadius: "var(--admiral-radius-button)",
            border: "none",
            backgroundColor: isDryRun ? "var(--color-primary)" : "var(--color-success)",
            color: "#fff",
            cursor: disabled ? "not-allowed" : "pointer",
            opacity: disabled ? 0.5 : 1,
          }}
        >
          {isDryRun ? "开始 Test Run" : "部署上线"}
        </button>
      </div>
    </div>
  );
}
