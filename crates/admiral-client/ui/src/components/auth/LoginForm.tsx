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

  const inputClass =
    "w-full border rounded px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-primary/30";
  const inputStyle = {
    borderColor: "var(--admiral-border)",
    borderRadius: "var(--admiral-radius-input)",
    color: "var(--admiral-text-primary)",
  };

  return (
    <div className="min-h-screen flex items-center justify-center p-4" style={{ backgroundColor: "var(--admiral-bg)" }}>
      <div
        className="w-full max-w-sm p-6 rounded-lg shadow"
        style={{
          backgroundColor: "var(--admiral-card)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "var(--admiral-shadow-card)",
        }}
      >
        <h1 className="text-xl font-semibold text-center mb-4" style={{ color: "var(--admiral-text-primary)" }}>
          Admiral
        </h1>
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
          <input
            type="email"
            placeholder="邮箱"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className={inputClass}
            style={inputStyle}
            required
          />
          <input
            type="password"
            placeholder="密码"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className={inputClass}
            style={inputStyle}
            required
          />
          {tab === "register" && (
            <>
              <input
                type="text"
                placeholder="姓名"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className={inputClass}
                style={inputStyle}
                required
              />
              <input
                type="text"
                placeholder="公司（选填）"
                value={company}
                onChange={(e) => setCompany(e.target.value)}
                className={inputClass}
                style={inputStyle}
              />
              <input
                type="text"
                placeholder="邀请码"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                className={inputClass}
                style={inputStyle}
                required
              />
            </>
          )}
          {error && <p className="text-danger text-sm">{error}</p>}
          {serverUrl && (
            <p className="text-xs" style={{ color: "var(--admiral-text-placeholder)" }} title="当前连接的服务端地址，若错误请修改客户端配置文件后重启">
              服务端: {serverUrl}
            </p>
          )}
          <button
            type="submit"
            disabled={loading}
            className="w-full py-2 text-white rounded disabled:opacity-50 bg-primary"
            style={{ borderRadius: "var(--admiral-radius-button)" }}
          >
            {loading ? "..." : tab === "login" ? "登录" : "注册"}
          </button>
        </form>
      </div>
    </div>
  );
}
