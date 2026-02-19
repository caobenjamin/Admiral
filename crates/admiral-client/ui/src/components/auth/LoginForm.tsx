import { useState } from "react";
import { useAuth } from "@/hooks/useAuth";

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
  const { login, register } = useAuth();

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

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 p-4">
      <div className="w-full max-w-sm bg-white rounded-lg shadow p-6">
        <h1 className="text-xl font-semibold text-center mb-4">Admiral</h1>
        <div className="flex border-b mb-4">
          <button
            type="button"
            className={`flex-1 py-2 text-sm ${tab === "login" ? "border-b-2 border-blue-600 text-blue-600" : ""}`}
            onClick={() => setTab("login")}
          >
            登录
          </button>
          <button
            type="button"
            className={`flex-1 py-2 text-sm ${tab === "register" ? "border-b-2 border-blue-600 text-blue-600" : ""}`}
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
            className="w-full border rounded px-3 py-2"
            required
          />
          <input
            type="password"
            placeholder="密码"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="w-full border rounded px-3 py-2"
            required
          />
          {tab === "register" && (
            <>
              <input
                type="text"
                placeholder="姓名"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full border rounded px-3 py-2"
                required
              />
              <input
                type="text"
                placeholder="公司（选填）"
                value={company}
                onChange={(e) => setCompany(e.target.value)}
                className="w-full border rounded px-3 py-2"
              />
              <input
                type="text"
                placeholder="邀请码"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value)}
                className="w-full border rounded px-3 py-2"
                required
              />
            </>
          )}
          {error && <p className="text-red-600 text-sm">{error}</p>}
          <button
            type="submit"
            disabled={loading}
            className="w-full bg-blue-600 text-white rounded py-2 disabled:opacity-50"
          >
            {loading ? "..." : tab === "login" ? "登录" : "注册"}
          </button>
        </form>
      </div>
    </div>
  );
}
