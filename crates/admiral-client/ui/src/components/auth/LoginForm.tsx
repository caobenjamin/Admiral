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
    padding: "8px 12px",
    fontSize: 14,
    color: "var(--admiral-text-primary)",
    outline: "none",
  };
  const inputError = {
    borderColor: "var(--color-danger)",
    boxShadow: "0 0 0 2px rgba(245, 108, 108, 0.3)",
  };
  const inputFocus = {
    borderColor: "var(--color-primary)",
    boxShadow: "0 0 0 2px rgba(64, 158, 255, 0.3)",
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ backgroundColor: "var(--admiral-bg)" }}>
      <div
        className={`w-full max-w-sm p-6 rounded-lg shadow ${error ? "login-shake" : ""}`}
        style={{
          backgroundColor: "var(--admiral-card)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "var(--admiral-shadow-card)",
        }}
      >
        <div className="text-center mb-6">
          <h1 className="text-xl font-semibold mb-1" style={{ color: "var(--admiral-text-primary)", fontSize: 18 }}>
            ⚓ Admiral
          </h1>
          <p className="text-sm" style={{ color: "var(--admiral-text-secondary)" }}>
            广告智能优化平台
          </p>
        </div>
        <div className="flex mb-4" style={{ borderBottomColor: "var(--admiral-border)", borderBottomWidth: 1 }}>
          <button
            type="button"
            className={`flex-1 py-2 text-sm ${tab === "login" ? "border-b-2 text-primary" : ""}`}
            style={tab === "login" ? { borderBottomColor: "var(--color-primary)" } : { color: "var(--admiral-text-secondary)" }}
            onClick={() => setTab("login")}
          >
            登录
          </button>
          <button
            type="button"
            className={`flex-1 py-2 text-sm ${tab === "register" ? "border-b-2 text-primary" : ""}`}
            style={tab === "register" ? { borderBottomColor: "var(--color-primary)" } : { color: "var(--admiral-text-secondary)" }}
            onClick={() => setTab("register")}
          >
            注册
          </button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-3">
          <label className="block text-sm font-medium text-[var(--admiral-text-primary)] mb-1">邮箱</label>
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
          <label className="block text-sm font-medium text-[var(--admiral-text-primary)] mb-1">密码</label>
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
          {tab === "register" && (
            <>
              <label className="block text-sm font-medium text-[var(--admiral-text-primary)] mb-1">姓名</label>
              <input
                type="text"
                placeholder="请输入姓名"
                value={name}
                onChange={(e) => setName(e.target.value)}
                style={inputBase}
                required
              />
              <label className="block text-sm font-medium text-[var(--admiral-text-primary)] mb-1">公司（选填）</label>
              <input
                type="text"
                placeholder="请输入公司"
                value={company}
                onChange={(e) => setCompany(e.target.value)}
                style={inputBase}
              />
              <label className="block text-sm font-medium text-[var(--admiral-text-primary)] mb-1">邀请码</label>
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
            </>
          )}
          {error && <p className="text-sm" style={{ color: "var(--color-danger)" }}>{error}</p>}
          {serverUrl && (
            <p className="text-xs" style={{ color: "var(--admiral-text-placeholder)" }} title="当前连接的服务端地址，若错误请修改客户端配置文件后重启">
              服务端: {serverUrl}
            </p>
          )}
          <button
            type="submit"
            disabled={loading}
            className="w-full py-2.5 text-white rounded disabled:opacity-50 bg-[var(--color-primary)] hover:opacity-90 transition-opacity"
            style={{ borderRadius: "var(--admiral-radius-button)", fontSize: 14, fontWeight: 500 }}
          >
            {loading ? "提交中…" : tab === "login" ? "登 录" : "注 册"}
          </button>
        </form>
      </div>
    </div>
  );
}
