import { useState, useCallback, useEffect } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";

export function TaskCreate() {
  const [name, setName] = useState("");
  const [type, setType] = useState<"instant" | "scheduled">("instant");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const { createTask } = useTaskStore();
  const navigate = useNavigate();

  const handleCreate = useCallback(async () => {
    if (!name.trim()) {
      setError("请输入任务名称");
      return;
    }
    setError("");
    setLoading(true);
    try {
      const task = await createTask({ name: name.trim(), type });
      navigate("/", { replace: true });
      useUiStore.getState().setSelectedTaskId(task.id);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [name, type, createTask, navigate]);

  useEffect(() => {
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (window.confirm("放弃创建任务？")) navigate("/");
      }
    };
    window.addEventListener("keydown", onEsc);
    return () => window.removeEventListener("keydown", onEsc);
  }, [navigate]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && name.trim()) handleCreate();
    },
    [handleCreate, name]
  );

  return (
    <div style={{ maxWidth: 480, margin: "0 auto", padding: 24 }}>
      <div
        style={{
          backgroundColor: "var(--admiral-card)",
          border: "1px solid var(--admiral-border)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "var(--admiral-shadow-card)",
          padding: 24,
        }}
      >
        <h2 style={{ fontSize: 18, fontWeight: 600, color: "var(--admiral-text-primary)", margin: "0 0 20px" }}>
          新建任务
        </h2>

        <label style={{ display: "block", fontSize: 14, fontWeight: 500, color: "var(--admiral-text-primary)", marginBottom: 8 }}>
          任务名称
        </label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="例如：B站广告出价优化"
          className="admiral-field"
          style={{ width: "100%", padding: "8px 12px", fontSize: 14, marginBottom: 16 }}
          autoFocus
        />

        <label style={{ display: "block", fontSize: 14, fontWeight: 500, color: "var(--admiral-text-primary)", marginBottom: 12 }}>
          任务类型
        </label>
        <div style={{ display: "flex", gap: 12, marginBottom: 20 }}>
          <button
            type="button"
            onClick={() => setType("instant")}
            style={{
              flex: 1,
              padding: 16,
              borderRadius: "var(--admiral-radius-card)",
              border: type === "instant" ? "2px solid var(--color-primary)" : "1px solid var(--admiral-border)",
              backgroundColor: type === "instant" ? "rgba(64, 158, 255, 0.08)" : "var(--admiral-card)",
              color: type === "instant" ? "var(--color-primary)" : "var(--admiral-text-primary)",
              cursor: "pointer",
              textAlign: "left",
            }}
          >
            <span style={{ fontWeight: 600, display: "block" }}>手动触发</span>
            <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>需要时手动执行</span>
          </button>
          <button
            type="button"
            onClick={() => setType("scheduled")}
            style={{
              flex: 1,
              padding: 16,
              borderRadius: "var(--admiral-radius-card)",
              border: type === "scheduled" ? "2px solid var(--color-primary)" : "1px solid var(--admiral-border)",
              backgroundColor: type === "scheduled" ? "rgba(64, 158, 255, 0.08)" : "var(--admiral-card)",
              color: type === "scheduled" ? "var(--color-primary)" : "var(--admiral-text-primary)",
              cursor: "pointer",
              textAlign: "left",
            }}
          >
            <span style={{ fontWeight: 600, display: "block" }}>定时执行</span>
            <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>按计划自动运行</span>
          </button>
        </div>

        <p style={{ fontSize: 12, color: "var(--admiral-text-placeholder)", marginBottom: 16 }}>
          创建后，系统会引导你通过对话逐步完善任务配置
        </p>

        {error && (
          <p style={{ color: "var(--color-danger)", fontSize: 14, marginBottom: 12 }}>{error}</p>
        )}

        <button
          type="button"
          onClick={handleCreate}
          disabled={loading || !name.trim()}
          className="btn btn-primary"
          style={{ width: "100%" }}
        >
          {loading ? "创建中..." : "创建任务"}
        </button>
      </div>

      <Link
        to="/"
        style={{
          display: "inline-block",
          marginTop: 16,
          fontSize: 14,
          color: "var(--admiral-text-secondary)",
          textDecoration: "none",
        }}
      >
        返回首页
      </Link>
    </div>
  );
}
