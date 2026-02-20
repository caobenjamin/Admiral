import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import type { TokenMeta, OAuthAccountsResponse } from "@/lib/types";

export function PlatformAuth() {
  const [status, setStatus] = useState<TokenMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [accounts, setAccounts] = useState<OAuthAccountsResponse | null>(null);
  const [accountsLoading, setAccountsLoading] = useState(false);
  /** 当前可绑定列表对应的授权（用于提交绑定时的 oauth_token_id） */
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  const [bindLoading, setBindLoading] = useState(false);

  const loadStatus = async () => {
    setLoading(true);
    setError("");
    try {
      const list = await tauri.oauthStatus();
      setStatus(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStatus();
  }, []);

  const handleInitiate = async () => {
    setError("");
    try {
      const { authorization_url } = await tauri.oauthInitiate();
      await invoke("open_url", { url: authorization_url });
      setTimeout(loadStatus, 2000);
    } catch (e) {
      setError(String(e));
    }
  };

  /** 获取可绑定账户列表：指定某次授权（oauth_session_id） */
  const loadAccountsForSession = async (sessionId: string) => {
    setSelectedSessionId(sessionId);
    setAccounts(null);
    setError("");
    setAccountsLoading(true);
    try {
      const data = await tauri.oauthAccountsList(sessionId);
      setAccounts(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setAccountsLoading(false);
    }
  };

  /** 获取可绑定账户列表：默认最近一次授权（不传 oauth_session_id） */
  const loadAccountsLatest = async () => {
    setSelectedSessionId(null);
    setAccounts(null);
    setError("");
    setAccountsLoading(true);
    try {
      const data = await tauri.oauthAccountsList();
      setAccounts(data);
      if (status.length > 0) setSelectedSessionId(status[0].id);
    } catch (e) {
      setError(String(e));
    } finally {
      setAccountsLoading(false);
    }
  };

  const handleBind = async () => {
    const tokenId = selectedSessionId ?? status[0]?.id;
    if (!tokenId || selectedAccountIds.length === 0) return;
    setBindLoading(true);
    setError("");
    try {
      await tauri.oauthBindAccounts(tokenId, selectedAccountIds);
      setSelectedAccountIds([]);
      setAccounts(null);
      setSelectedSessionId(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBindLoading(false);
    }
  };

  const toggleAccount = (id: number) => {
    setSelectedAccountIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  const selectAllAccounts = () => {
    if (!accounts) return;
    setSelectedAccountIds(accounts.records.map((r) => r.account_id));
  };
  const clearSelection = () => setSelectedAccountIds([]);

  return (
    <div className="space-y-6 max-w-2xl p-4">
      <h2 className="text-lg font-semibold" style={{ color: "var(--admiral-text-primary)" }}>B 站授权</h2>
      {error && (
        <div
          className="rounded-lg px-3 py-2 text-sm text-danger border"
          style={{ backgroundColor: "rgba(245,108,108,0.1)", borderColor: "var(--color-danger)" }}
        >
          {error}
        </div>
      )}

      <section>
        <h3 className="text-sm font-medium mb-2" style={{ color: "var(--admiral-text-primary)" }}>发起授权</h3>
        <button
          type="button"
          onClick={handleInitiate}
          className="rounded-lg border px-4 py-2 text-sm"
          style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", color: "var(--admiral-text-primary)" }}
        >
          发起授权（打开浏览器）
        </button>
      </section>

      <section>
        <h3 className="text-sm font-medium mb-2" style={{ color: "var(--admiral-text-primary)" }}>授权列表</h3>
        {loading ? (
          <p className="text-sm text-gray-500">加载中...</p>
        ) : status.length === 0 ? (
          <p className="text-sm text-gray-500">暂无授权，请先发起授权。</p>
        ) : (
          <ul className="space-y-2">
            {status.map((s) => (
              <li
                key={s.id}
                className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2"
                style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", borderRadius: "var(--admiral-radius-card)" }}
              >
                <span className="text-sm" style={{ color: "var(--admiral-text-primary)" }}>
                  {s.platform} · {s.created_at}
                </span>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => loadAccountsForSession(s.id)}
                    disabled={accountsLoading}
                    className="text-sm text-primary hover:underline disabled:opacity-50"
                  >
                    获取可绑定账户
                  </button>
                  <button
                    type="button"
                    onClick={async () => {
                      try {
                        await tauri.oauthDeleteSession(s.id);
                        loadStatus();
                        if (selectedSessionId === s.id) {
                          setAccounts(null);
                          setSelectedSessionId(null);
                        }
                      } catch (e) {
                        setError(String(e));
                      }
                    }}
                    className="text-sm text-danger hover:underline"
                  >
                    取消授权
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
        {status.length > 0 && (
          <button
            type="button"
            onClick={loadAccountsLatest}
            disabled={accountsLoading}
            className="mt-2 text-sm text-gray-600 hover:text-gray-800 disabled:opacity-50"
          >
            获取可绑定账户列表（使用最近一次授权）
          </button>
        )}
      </section>

      {accountsLoading && (
        <p className="text-sm text-gray-500">正在获取可绑定账户列表…</p>
      )}

      {accounts && !accountsLoading && (
        <section
          className="rounded-lg border p-4"
          style={{ borderColor: "var(--admiral-border)", backgroundColor: "var(--admiral-card)", borderRadius: "var(--admiral-radius-card)" }}
        >
          <h3 className="text-sm font-medium mb-2" style={{ color: "var(--admiral-text-primary)" }}>可绑定账户列表</h3>
          <p className="text-xs mb-3" style={{ color: "var(--admiral-text-secondary)" }}>
            共 {accounts.total} 个账户，勾选后点击「提交绑定」。
          </p>
          <div className="mb-3 flex gap-2">
            <button
              type="button"
              onClick={selectAllAccounts}
              className="text-sm text-primary hover:underline"
            >
              全选
            </button>
            <button
              type="button"
              onClick={clearSelection}
              className="text-sm hover:underline"
              style={{ color: "var(--admiral-text-secondary)" }}
            >
              清空
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="border-b border-gray-200 text-left text-gray-500">
                  <th className="py-2 pr-2 w-10">选择</th>
                  <th className="py-2 pr-2">账户 ID</th>
                  <th className="py-2 pr-2">账户名称</th>
                  <th className="py-2">角色</th>
                </tr>
              </thead>
              <tbody>
                {accounts.records.map((r) => (
                  <tr key={r.account_id} className="border-b border-gray-100">
                    <td className="py-2 pr-2">
                      <input
                        type="checkbox"
                        checked={selectedAccountIds.includes(r.account_id)}
                        onChange={() => toggleAccount(r.account_id)}
                        className="rounded border-gray-300"
                      />
                    </td>
                    <td className="py-2 pr-2 font-mono">{r.account_id}</td>
                    <td className="py-2 pr-2">{r.account_name ?? "—"}</td>
                    <td className="py-2">{r.account_role ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="mt-4 flex items-center gap-2">
            <button
              type="button"
              onClick={handleBind}
              disabled={bindLoading || selectedAccountIds.length === 0}
              className="rounded-lg px-4 py-2 text-white text-sm font-medium bg-primary disabled:opacity-50"
              style={{ borderRadius: "var(--admiral-radius-button)" }}
            >
              {bindLoading ? "提交中…" : "提交绑定"}
            </button>
            <span className="text-xs text-gray-500">
              已选 {selectedAccountIds.length} 个账户
            </span>
          </div>
        </section>
      )}
    </div>
  );
}
