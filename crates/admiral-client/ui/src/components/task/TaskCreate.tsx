// PRD 12.4: 任务创建流程（向导式）
// 规范 §5.3 新建任务

import { useState, useEffect, useCallback } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useTaskStore } from "@/stores/taskStore";
import { useUiStore } from "@/stores/uiStore";

type Step = "name" | "type" | "confirm";

const STEPS: Step[] = ["name", "type", "confirm"];
const STEP_LABELS: Record<Step, string> = {
  name: "任务名称",
  type: "任务类型",
  confirm: "确认创建",
};

export function TaskCreate() {
  const [step, setStep] = useState<Step>("name");
  const [name, setName] = useState("");
  const [type, setType] = useState<"instant" | "scheduled">("instant");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const { createTask } = useTaskStore();
  const navigate = useNavigate();

  const stepIndex = STEPS.indexOf(step);

  const handleCreate = useCallback(async () => {
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
        if (stepIndex > 0) {
          setStep(STEPS[stepIndex - 1]);
        } else if (window.confirm("放弃创建任务？")) {
          navigate("/");
        }
      }
    };
    window.addEventListener("keydown", onEsc);
    return () => window.removeEventListener("keydown", onEsc);
  }, [stepIndex, navigate]);

  const inputBase = {
    width: "100%",
    padding: "8px 12px",
    fontSize: 14,
    borderRadius: "var(--admiral-radius-input)",
    border: "1px solid var(--admiral-border)",
    color: "var(--admiral-text-primary)",
    outline: "none",
  };
  const inputFocus = {
    borderColor: "var(--color-primary)",
    boxShadow: "0 0 0 2px rgba(64, 158, 255, 0.3)",
  };

  return (
    <div
      style={{
        maxWidth: 480,
        margin: "0 auto",
        padding: 24,
      }}
    >
      <div
        style={{
          backgroundColor: "var(--admiral-card)",
          border: "1px solid var(--admiral-border)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "var(--admiral-shadow-card)",
          padding: 24,
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
          <h2 style={{ fontSize: 18, fontWeight: 600, color: "var(--admiral-text-primary)", margin: 0 }}>
            新建任务
          </h2>
          <span style={{ fontSize: 14, color: "var(--admiral-text-placeholder)" }}>
            {stepIndex + 1}/{STEPS.length}
          </span>
        </div>

        {/* 步骤指示器 */}
        <div
          style={{
            display: "flex",
            gap: 8,
            marginBottom: 24,
            fontSize: 12,
            color: "var(--admiral-text-secondary)",
          }}
        >
          {STEPS.map((s, i) => (
            <span key={s}>
              <span style={{ fontWeight: i === stepIndex ? 600 : 400, color: i <= stepIndex ? "var(--admiral-text-primary)" : undefined }}>
                ① {STEP_LABELS[s]}
              </span>
              {i < STEPS.length - 1 && <span style={{ marginLeft: 8 }}>——</span>}
            </span>
          ))}
        </div>

        {step === "name" && (
          <>
            <label style={{ display: "block", fontSize: 14, fontWeight: 500, color: "var(--admiral-text-primary)", marginBottom: 8 }}>
              任务名称
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onFocus={(e) => {
                e.target.style.borderColor = inputFocus.borderColor as string;
                e.target.style.boxShadow = inputFocus.boxShadow as string;
              }}
              onBlur={(e) => {
                e.target.style.borderColor = inputBase.border as string;
                e.target.style.boxShadow = "none";
              }}
              placeholder="例如：B站广告出价优化"
              style={inputBase}
              autoFocus
            />
            <p style={{ fontSize: 12, color: "var(--admiral-text-placeholder)", marginTop: 8 }}>
              为任务起一个描述性的名称
            </p>
            <div style={{ marginTop: 24 }}>
              <button
                type="button"
                onClick={() => {
                  if (!name.trim()) setError("请输入任务名称");
                  else {
                    setError("");
                    setStep("type");
                  }
                }}
                disabled={!name.trim()}
                style={{
                  padding: "10px 20px",
                  fontSize: 14,
                  fontWeight: 500,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "none",
                  backgroundColor: "var(--color-primary)",
                  color: "#fff",
                  cursor: name.trim() ? "pointer" : "not-allowed",
                  opacity: name.trim() ? 1 : 0.5,
                }}
              >
                下一步 →
              </button>
            </div>
          </>
        )}

        {step === "type" && (
          <>
            <label style={{ display: "block", fontSize: 14, fontWeight: 500, color: "var(--admiral-text-primary)", marginBottom: 12 }}>
              选择任务类型
            </label>
            <div style={{ display: "flex", gap: 12, marginBottom: 24 }}>
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
                <span style={{ fontSize: 18, marginBottom: 4, display: "block" }}>⚡</span>
                <span style={{ fontWeight: 600 }}>即时</span>
                <br />
                <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>立即执行</span>
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
                <span style={{ fontSize: 18, marginBottom: 4, display: "block" }}>🕐</span>
                <span style={{ fontWeight: 600 }}>定时</span>
                <br />
                <span style={{ fontSize: 12, color: "var(--admiral-text-secondary)" }}>按计划执行</span>
              </button>
            </div>
            <div style={{ display: "flex", gap: 12 }}>
              <button
                type="button"
                onClick={() => setStep("name")}
                style={{
                  padding: "8px 16px",
                  fontSize: 14,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "1px solid var(--admiral-border)",
                  backgroundColor: "var(--admiral-card)",
                  color: "var(--admiral-text-primary)",
                  cursor: "pointer",
                }}
              >
                ← 上一步
              </button>
              <button
                type="button"
                onClick={() => setStep("confirm")}
                style={{
                  padding: "10px 20px",
                  fontSize: 14,
                  fontWeight: 500,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "none",
                  backgroundColor: "var(--color-primary)",
                  color: "#fff",
                  cursor: "pointer",
                }}
              >
                下一步 →
              </button>
            </div>
          </>
        )}

        {step === "confirm" && (
          <>
            <p style={{ fontSize: 14, color: "var(--admiral-text-primary)", marginBottom: 16 }}>确认任务信息</p>
            <div style={{ marginBottom: 24, fontSize: 14, color: "var(--admiral-text-secondary)" }}>
              <p>名称：{name || "—"}</p>
              <p>类型：{type === "instant" ? "即时任务" : "定时任务"}</p>
            </div>
            {error && (
              <p style={{ color: "var(--color-danger)", fontSize: 14, marginBottom: 12 }}>{error}</p>
            )}
            <div style={{ display: "flex", gap: 12 }}>
              <button
                type="button"
                onClick={() => setStep("type")}
                style={{
                  padding: "8px 16px",
                  fontSize: 14,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "1px solid var(--admiral-border)",
                  backgroundColor: "var(--admiral-card)",
                  color: "var(--admiral-text-primary)",
                  cursor: "pointer",
                }}
              >
                ← 上一步
              </button>
              <button
                type="button"
                onClick={handleCreate}
                disabled={loading}
                style={{
                  padding: "10px 20px",
                  fontSize: 14,
                  fontWeight: 500,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "none",
                  backgroundColor: "var(--color-primary)",
                  color: "#fff",
                  cursor: loading ? "not-allowed" : "pointer",
                  opacity: loading ? 0.7 : 1,
                }}
              >
                {loading ? "创建中…" : "创建任务"}
              </button>
            </div>
          </>
        )}
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
        ← 返回首页
      </Link>
    </div>
  );
}
