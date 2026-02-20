import { useEffect } from "react";
import { useToastStore } from "@/stores/toastStore";

const TOAST_COLORS: Record<string, React.CSSProperties> = {
  success: { backgroundColor: "var(--color-success)", color: "#fff" },
  error: { backgroundColor: "var(--color-danger)", color: "#fff" },
  warning: { backgroundColor: "var(--color-warning)", color: "#fff" },
  info: { backgroundColor: "var(--color-primary)", color: "#fff" },
};

export function Toast() {
  const { toasts, dismiss } = useToastStore();

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && toasts.length > 0) {
        dismiss(toasts[toasts.length - 1]!.id);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toasts, dismiss]);

  if (toasts.length === 0) return null;

  return (
    <div
      role="alert"
      aria-live="assertive"
      style={{
        position: "fixed",
        top: 16,
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 200,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 8,
        pointerEvents: "none",
      }}
    >
      {toasts.map((t) => (
        <div
          key={t.id}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            padding: "10px 16px",
            borderRadius: "var(--admiral-radius-button)",
            fontSize: 14,
            fontWeight: 500,
            boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
            ...TOAST_COLORS[t.type],
            pointerEvents: "auto",
            animation: "toastIn 0.25s ease-out",
          }}
        >
          <span>{t.message}</span>
          <button
            type="button"
            onClick={() => dismiss(t.id)}
            style={{
              background: "none",
              border: "none",
              color: "inherit",
              cursor: "pointer",
              padding: 4,
              opacity: 0.9,
            }}
            aria-label="关闭"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
