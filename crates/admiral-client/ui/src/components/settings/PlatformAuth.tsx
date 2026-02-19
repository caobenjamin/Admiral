import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import type { TokenMeta } from "@/lib/types";

export function PlatformAuth() {
  const [status, setStatus] = useState<TokenMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [accounts, setAccounts] = useState<{ records: { account_id: number; account_name?: string }[]; total: number } | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");
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

  const loadAccounts = async (sessionId: string) => {
    setSelectedSessionId(sessionId);
    setError("");
    try {
      const data = await tauri.oauthAccountsList(sessionId);
      setAccounts(data as { records: { account_id: number; account_name?: string }[]; total: number });
    } catch (e) {
      setError(String(e));
    }
  };

  const handleBind = async () => {
    if (!selectedSessionId || selectedAccountIds.length === 0) return;
    setBindLoading(true);
    setError("");
    try {
      await tauri.oauthBindAccounts(selectedSessionId, selectedAccountIds);
      setSelectedAccountIds([]);
      setAccounts(null);
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

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-medium">B 站授权</h2>
      {error && <p className="text-red-600 text-sm">{error}</p>}
      <button
        type="button"
        onClick={handleInitiate}
        className="rounded border px-4 py-2 bg-blue-600 text-white"
      >
        发起授权（打开浏览器）
      </button>
      <div>
        <h3 className="font-medium mb-2">授权列表</h3>
        {loading ? (
          <p className="text-sm text-gray-500">加载中...</p>
        ) : (
          <ul className="space-y-1 text-sm">
            {status.map((s) => (
              <li key={s.id} className="flex items-center gap-2">
                <span>{s.platform} - {s.created_at}</span>
                <button
                  type="button"
                  onClick={() => loadAccounts(s.id)}
                  className="text-blue-600 hover:underline"
                >
                  绑定账户
                </button>
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      await tauri.oauthDeleteSession(s.id);
                      loadStatus();
                    } catch (e) {
                      setError(String(e));
                    }
                  }}
                  className="text-red-600"
                >
                  取消授权
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      {accounts && (
        <div>
          <h3 className="font-medium mb-2">选择要绑定的账户（Session: {selectedSessionId}）</h3>
          <ul className="space-y-1 mb-2">
            {accounts.records.map((r) => (
              <li key={r.account_id} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={selectedAccountIds.includes(r.account_id)}
                  onChange={() => toggleAccount(r.account_id)}
                />
                <span>{r.account_id} - {r.account_name ?? ""}</span>
              </li>
            ))}
          </ul>
          <button
            type="button"
            onClick={handleBind}
            disabled={bindLoading || !selectedSessionId}
            className="rounded border px-4 py-2 bg-blue-600 text-white disabled:opacity-50"
          >
            提交绑定
          </button>
        </div>
      )}
    </div>
  );
}
