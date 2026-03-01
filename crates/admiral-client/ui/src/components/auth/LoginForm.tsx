import { useState, useEffect } from "react";
import { useAuth } from "@/hooks/useAuth";
import * as tauri from "@/lib/tauri";

type Tab = "login" | "register";

export function LoginForm() {
  const [tab, setTab] = useState<Tab>("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [serverUrl, setServerUrl] = useState<string>("");
  const { login, register } = useAuth();

  useEffect(() => {
    tauri.getServerUrl().then(setServerUrl).catch(() => setServerUrl(""));
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      if (tab === "login") {
        await login(email, password);
      } else {
        await register(email, password, name, company, inviteCode);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const inputBase = {
    width: "100%",
    border: "1px solid var(--admiral-border)",
    borderRadius: "var(--admiral-radius-input)",
    padding: "9px 12px",
    fontSize: 14,
    color: "var(--admiral-text-primary)",
    outline: "none",
    backgroundColor: "#ffffff",
  };
  const inputError = {
    borderColor: "var(--color-danger)",
    boxShadow: "0 0 0 2px rgba(220, 38, 38, 0.16)",
  };
  const inputFocus = {
    borderColor: "var(--color-primary)",
    boxShadow: "0 0 0 2px rgba(15, 108, 189, 0.16)",
  };

  return (
    <div className="auth-page">
      <section className="auth-hero">
        <div style={{ display: "inline-flex", alignItems: "center", gap: 8, marginBottom: 10 }}>
          <span className="admiral-chip admiral-chip-primary" style={{ background: "rgba(255,255,255,0.14)", color: "#eef7ff", borderColor: "rgba(255,255,255,0.25)" }}>
            Admiral v1.0
          </span>
        </div>
        <h1 style={{ fontSize: 30, lineHeight: 1.25, fontWeight: 700, margin: 0 }}>广告操作的智能指挥系统</h1>
        <p style={{ margin: "12px 0 0", fontSize: 14, opacity: 0.92 }}>
          通过自然语言任务编排、Dry Run/Test Run 分级确认、Agent 对话追踪，稳定完成广告执行闭环。
        </p>
        <div style={{ marginTop: 20, display: "grid", gap: 10 }}>
          <div style={{ padding: "10px 12px", borderRadius: 10, backgroundColor: "rgba(255,255,255,0.14)", border: "1px solid rgba(255,255,255,0.2)" }}>
            任务全流程可追溯：规划、预演、测试、部署
          </div>
          <div style={{ padding: "10px 12px", borderRadius: 10, backgroundColor: "rgba(255,255,255,0.14)", border: "1px solid rgba(255,255,255,0.2)" }}>
            平台授权与账户绑定分层管理
          </div>
          <div style={{ padding: "10px 12px", borderRadius: 10, backgroundColor: "rgba(255,255,255,0.14)", border: "1px solid rgba(255,255,255,0.2)" }}>
            本地执行引擎，关键状态透明可控
          </div>
        </div>
      </section>

      <section className={`auth-card ${error ? "login-shake" : ""}`}>
        <div style={{ marginBottom: 14 }}>
          <h2 style={{ margin: 0, fontSize: 20, fontWeight: 700, color: "var(--admiral-text-primary)" }}>账号访问</h2>
          <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>登录或注册后进入 Admiral 工作台</p>
        </div>

        <div className="auth-tabs">
          <button
            type="button"
            className={`auth-tab ${tab === "login" ? "auth-tab-active" : ""}`}
            onClick={() => setTab("login")}
          >
            登录
          </button>
          <button
            type="button"
            className={`auth-tab ${tab === "register" ? "auth-tab-active" : ""}`}
            onClick={() => setTab("register")}
          >
            注册
          </button>
        </div>

        <form onSubmit={handleSubmit} style={{ display: "grid", gap: 10 }}>
          <label style={{ display: "grid", gap: 6 }}>
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>邮箱</span>
          <input
            type="email"
            placeholder="请输入邮箱"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            style={{
              ...inputBase,
              ...(error ? inputError : {}),
            }}
            onFocus={(e) => {
              if (!error) {
                e.target.style.borderColor = inputFocus.borderColor as string;
                e.target.style.boxShadow = inputFocus.boxShadow as string;
              }
            }}
            onBlur={(e) => {
              e.target.style.borderColor = "var(--admiral-border)";
              e.target.style.boxShadow = "none";
            }}
            required
          />
          </label>
          <label style={{ display: "grid", gap: 6 }}>
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>密码</span>
          <input
            type="password"
            placeholder="请输入密码（至少 6 位）"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            style={{
              ...inputBase,
              ...(error ? inputError : {}),
            }}
            onFocus={(e) => {
              if (!error) {
                e.target.style.borderColor = inputFocus.borderColor as string;
                e.target.style.boxShadow = inputFocus.boxShadow as string;
              }
            }}
            onBlur={(e) => {
              e.target.style.borderColor = "var(--admiral-border)";
              e.target.style.boxShadow = "none";
            }}
            required
            minLength={6}
          />
          </label>
          {tab === "register" && (
            <>
              <label style={{ display: "grid", gap: 6 }}>
                <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>姓名</span>
              <input
                type="text"
                placeholder="请输入姓名"
                value={name}
                onChange={(e) => setName(e.target.value)}
                style={inputBase}
                required
              />
              </label>
              <label style={{ display: "grid", gap: 6 }}>
                <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>公司（选填）</span>
              <input
                type="text"
                placeholder="请输入公司"
                value={company}
                onChange={(e) => setCompany(e.target.value)}
                style={inputBase}
              />
              </label>
              <label style={{ display: "grid", gap: 6 }}>
                <span style={{ fontSize: 13, fontWeight: 600, color: "var(--admiral-text-primary)" }}>邀请码</span>
              <input
                type="text"
                placeholder="请输入邀请码"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                style={{
                  ...inputBase,
                  ...(error && error.includes("邀请码") ? inputError : {}),
                }}
                required
              />
              </label>
            </>
          )}

          {error && (
            <div
              style={{
                padding: "8px 10px",
                borderRadius: 10,
                border: "1px solid rgba(220, 38, 38, 0.4)",
                backgroundColor: "#fef2f2",
                color: "var(--color-danger)",
                fontSize: 13,
              }}
            >
              {error}
            </div>
          )}

          {serverUrl && (
            <p style={{ margin: 0, fontSize: 12, color: "var(--admiral-text-placeholder)" }} title="当前连接的服务端地址，若错误请修改客户端配置文件后重启">
              服务端: {serverUrl}
            </p>
          )}

          <button type="submit" disabled={loading} className="btn btn-primary btn-large" style={{ width: "100%", marginTop: 4 }}>
            {loading ? "提交中..." : tab === "login" ? "登 录" : "注 册"}
          </button>
        </form>
      </section>
    </div>
  );
}
