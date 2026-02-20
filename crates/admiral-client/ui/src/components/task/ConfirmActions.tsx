interface ConfirmActionsProps {
  stage: "dry_run_passed" | "test_run_passed";
  onConfirm: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

export function ConfirmActions({ stage, onConfirm, onCancel, disabled }: ConfirmActionsProps) {
  const isDryRun = stage === "dry_run_passed";
  return (
    <div className="flex items-center gap-2 p-3 rounded-lg border" style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-bg)", borderRadius: "var(--admiral-radius-card)" }}>
      <span className="text-sm text-[var(--admiral-text-secondary)] flex-1">
        {isDryRun ? "Dry Run 已通过，可开始 Test Run" : "Test Run 已通过，可部署"}
      </span>
      <button
        type="button"
        onClick={onConfirm}
        disabled={disabled}
        className="px-4 py-2 text-sm font-medium text-white rounded-md bg-primary hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
        style={{ borderRadius: "var(--admiral-radius-button)" }}
      >
        {isDryRun ? "开始 Test Run" : "部署"}
      </button>
      <button
        type="button"
        onClick={onCancel}
        disabled={disabled}
        className="px-4 py-2 text-sm font-medium rounded-md text-danger hover:bg-danger/10 disabled:opacity-50 disabled:cursor-not-allowed"
        style={{ borderRadius: "var(--admiral-radius-button)" }}
      >
        取消
      </button>
    </div>
  );
}
