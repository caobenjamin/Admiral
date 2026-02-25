import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import { useToastStore } from "@/stores/toastStore";
import type {
  TokenMeta,
  OAuthAccountsResponse,
  BoundAccountRecord,
  BiliAdvertiserRecord,
} from "@/lib/types";

/** 确认解绑弹窗 */
function ConfirmUnbindDialog({
  open,
  accountName,
  onClose,
  onConfirm,
}: {
  open: boolean;
  accountName: string;
  onClose: () => void;
  onConfirm: () => void;
}) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="unbind-dialog-title"
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 100,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "rgba(0,0,0,0.3)",
      }}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="admiral-dialog-content"
        style={{
          backgroundColor: "var(--admiral-card)",
          border: "1px solid var(--admiral-border)",
          borderRadius: "var(--admiral-radius-card)",
          boxShadow: "0 8px 24px rgba(0,0,0,0.15)",
          maxWidth: 480,
          width: "min(90vw, 480px)",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            padding: "20px 24px",
            borderBottom: "1px solid var(--admiral-border)",
            fontSize: 16,
            fontWeight: 600,
            color: "var(--admiral-text-primary)",
          }}
          id="unbind-dialog-title"
        >
          确认解绑
        </div>
        <div style={{ padding: "20px 24px", color: "var(--admiral-text-primary)", fontSize: 14 }}>
          确定要解绑该账户{accountName ? `「${accountName}」` : ""}吗？
        </div>
        <div
          style={{
            padding: "12px 24px 20px",
            display: "flex",
            justifyContent: "flex-end",
            gap: 12,
          }}
        >
          <button type="button" onClick={onClose} className="btn btn-default">
            取消
          </button>
          <button type="button" onClick={onConfirm} className="btn btn-danger">
            确认解绑
          </button>
        </div>
      </div>
    </div>
  );
}

/** 新增账户弹窗：按 oauth_session_id 拉取该 token 下的账户列表 */
function BindAccountModal({
  open,
  onClose,
  oauthSessionId,
  onSuccess,
}: {
  open: boolean;
  onClose: () => void;
  oauthSessionId: string | null;
  onSuccess: () => void;
}) {
  const showToast = useToastStore((s) => s.show);
  const [accounts, setAccounts] = useState<OAuthAccountsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  const [bindLoading, setBindLoading] = useState(false);

  const loadAccounts = useCallback(async () => {
    if (!oauthSessionId) return;
    setLoading(true);
    setError("");
    try {
      const data = await tauri.oauthAccountsList(oauthSessionId);
      setAccounts(data);
      setSelectedAccountIds([]);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [oauthSessionId]);

  useEffect(() => {
    if (open && oauthSessionId) {
      loadAccounts();
    } else {
      setAccounts(null);
      setError("");
      setSelectedAccountIds([]);
    }
  }, [open, oauthSessionId, loadAccounts]);

  const handleBind = async () => {
    if (!oauthSessionId || selectedAccountIds.length === 0) return;
    setBindLoading(true);
    setError("");
    try {
      await tauri.oauthBindAccounts(oauthSessionId, selectedAccountIds);
      showToast("绑定成功", "success");
      onSuccess();
      onClose();
    } catch (e) {
      const msg = String(e);
      if (msg.includes("ACCOUNT_LIMIT") || msg.includes("账户数量上限")) {
        showToast(msg, "error");
      } else {
        setError(msg);
      }
    } finally {
      setBindLoading(false);
    }
  };

  const toggleAccount = (id: number) => {
    setSelectedAccountIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  const selectAll = () => {
    if (!accounts) return;
    setSelectedAccountIds(accounts.records.map((r) => r.account_id));
  };
  const clearSelection = () => setSelectedAccountIds([]);

  if (!open) return null;

  const cardStyle: React.CSSProperties = {
    backgroundColor: "var(--admiral-card)",
    border: "1px solid var(--admiral-border)",
    borderRadius: "var(--admiral-radius-card)",
    boxShadow: "var(--admiral-shadow-card)",
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="bind-modal-title"
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 100,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "rgba(0,0,0,0.3)",
      }}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="admiral-dialog-content"
        style={{
          ...cardStyle,
          width: "min(90vw, 560px)",
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            padding: "16px 20px",
            borderBottom: "1px solid var(--admiral-border)",
            fontSize: 16,
            fontWeight: 600,
            color: "var(--admiral-text-primary)",
          }}
          id="bind-modal-title"
        >
          新增账户
        </div>
        <div style={{ padding: 20, overflow: "auto", flex: 1 }}>
          {!oauthSessionId ? (
            <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14 }}>
              请先选择授权来源（BID / 用户名）。
            </p>
          ) : error ? (
            <div
              style={{
                padding: "10px 12px",
                fontSize: 14,
                color: "var(--color-danger)",
                backgroundColor: "rgba(245,108,108,0.1)",
                border: "1px solid var(--color-danger)",
                borderRadius: "var(--admiral-radius-input)",
              }}
            >
              {error}
            </div>
          ) : loading ? (
            <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14 }}>
              正在获取可绑定账户列表…
            </p>
          ) : accounts && accounts.records.length > 0 ? (
            <>
              <p
                style={{
                  fontSize: 13,
                  color: "var(--admiral-text-secondary)",
                  marginBottom: 12,
                }}
              >
                共 {accounts.total} 个账户，勾选需要绑定的账户后点击保存。
              </p>
              <div style={{ marginBottom: 12, display: "flex", gap: 12 }}>
                <button
                  type="button"
                  onClick={selectAll}
                  style={{
                    fontSize: 12,
                    color: "var(--color-primary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: 0,
                  }}
                >
                  全选
                </button>
                <button
                  type="button"
                  onClick={clearSelection}
                  style={{
                    fontSize: 12,
                    color: "var(--admiral-text-secondary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: 0,
                  }}
                >
                  清空
                </button>
              </div>
              <div style={{ overflowX: "auto" }}>
                <table
                  style={{
                    width: "100%",
                    fontSize: 14,
                    borderCollapse: "collapse",
                  }}
                >
                  <thead>
                    <tr
                      style={{
                        borderBottom: "1px solid var(--admiral-border)",
                        color: "var(--admiral-text-secondary)",
                        textAlign: "left",
                      }}
                    >
                      <th style={{ padding: "10px 12px 10px 0", width: 44 }}>选择</th>
                      <th style={{ padding: "10px 12px 10px 0" }}>账户 ID</th>
                      <th style={{ padding: "10px 12px 10px 0" }}>账户名称</th>
                      <th style={{ padding: "10px 0" }}>角色</th>
                    </tr>
                  </thead>
                  <tbody>
                    {accounts.records.map((r: BiliAdvertiserRecord) => (
                      <tr
                        key={r.account_id}
                        style={{
                          borderBottom: "1px solid var(--admiral-border)",
                        }}
                      >
                        <td style={{ padding: "10px 12px 10px 0" }}>
                          <input
                            type="checkbox"
                            checked={selectedAccountIds.includes(r.account_id)}
                            onChange={() => toggleAccount(r.account_id)}
                            style={{ width: 16, height: 16 }}
                          />
                        </td>
                        <td
                          style={{
                            padding: "10px 12px 10px 0",
                            fontFamily: "monospace",
                          }}
                        >
                          {r.account_id}
                        </td>
                        <td style={{ padding: "10px 12px 10px 0" }}>
                          {r.account_name ?? "—"}
                        </td>
                        <td style={{ padding: "10px 0" }}>{r.account_role ?? "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          ) : accounts && accounts.records.length === 0 ? (
            <p
              style={{ color: "var(--admiral-text-secondary)", fontSize: 14 }}
            >
              当前授权下无可绑定账户。
            </p>
          ) : null}
        </div>
        <div
          style={{
            padding: "12px 20px",
            borderTop: "1px solid var(--admiral-border)",
            display: "flex",
            justifyContent: "flex-end",
            gap: 12,
          }}
        >
          <button type="button" onClick={onClose} className="btn btn-default">
            取消
          </button>
          {oauthSessionId &&
            accounts &&
            accounts.records.length > 0 && (
              <button
                type="button"
                onClick={handleBind}
                disabled={bindLoading || selectedAccountIds.length === 0}
                className="btn btn-primary"
              >
                {bindLoading ? "保存中…" : `保存 (${selectedAccountIds.length})`}
              </button>
            )}
        </div>
      </div>
    </div>
  );
}

export function PlatformAuth() {
  const showToast = useToastStore((s) => s.show);
  const [status, setStatus] = useState<TokenMeta[]>([]);
  const [bindings, setBindings] = useState<BoundAccountRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [searchName, setSearchName] = useState("");
  const [searchId, setSearchId] = useState("");
  const [addAccountOpen, setAddAccountOpen] = useState(false);
  const [addAccountDropdownOpen, setAddAccountDropdownOpen] = useState(false);
  const [selectedTokenId, setSelectedTokenId] = useState<string | null>(null);
  const [unbindTarget, setUnbindTarget] = useState<BoundAccountRecord | null>(null);
  const [initiatePolling, setInitiatePolling] = useState(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const [statusList, bindingsList] = await Promise.all([
        tauri.oauthStatus(),
        tauri.oauthBindingsList().catch(() => [] as BoundAccountRecord[]),
      ]);
      setStatus(statusList);
      setBindings(bindingsList);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // 授权成功后轮询 status，检测到新 token 后自动刷新
  const prevStatusLenRef = useRef(0);
  useEffect(() => {
    if (!initiatePolling) return;
    prevStatusLenRef.current = status.length;
    const t = setInterval(async () => {
      const [statusList, bindingsList] = await Promise.all([
        tauri.oauthStatus().catch(() => [] as TokenMeta[]),
        tauri.oauthBindingsList().catch(() => [] as BoundAccountRecord[]),
      ]);
      setStatus(statusList);
      setBindings(bindingsList);
      if (statusList.length > prevStatusLenRef.current) {
        setInitiatePolling(false);
        showToast("授权成功", "success");
      }
      prevStatusLenRef.current = statusList.length;
    }, 2000);
    const timeout = setTimeout(() => {
      setInitiatePolling(false);
      showToast("授权超时，请重试", "error");
    }, 5 * 60 * 1000);
    return () => {
      clearInterval(t);
      clearTimeout(timeout);
    };
  }, [initiatePolling, status.length, showToast]);

  const handleInitiate = async () => {
    setError("");
    try {
      const { authorization_url } = await tauri.oauthInitiate();
      await invoke("open_url", { url: authorization_url });
      setInitiatePolling(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const filteredBindings = useMemo(() => {
    let list = bindings;
    if (searchName.trim()) {
      const kw = searchName.trim().toLowerCase();
      list = list.filter(
        (b) =>
          (b.account_name ?? "").toLowerCase().includes(kw)
      );
    }
    if (searchId.trim()) {
      const idStr = searchId.trim();
      list = list.filter((b) => String(b.account_id) === idStr);
    }
    return list;
  }, [bindings, searchName, searchId]);

  const handleUnbind = async () => {
    if (!unbindTarget) return;
    try {
      await tauri.oauthUnbindAccount(unbindTarget.account_id);
      showToast("解绑成功", "success");
      setUnbindTarget(null);
      loadData();
    } catch (e) {
      showToast(String(e), "error");
    }
  };

  const openBindModal = (tokenId: string) => {
    setSelectedTokenId(tokenId);
    setAddAccountDropdownOpen(false);
    setAddAccountOpen(true);
  };

  // 点击外部关闭下拉
  useEffect(() => {
    if (!addAccountDropdownOpen) return;
    const onDocClick = () => setAddAccountDropdownOpen(false);
    const t = setTimeout(() => document.addEventListener("click", onDocClick), 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener("click", onDocClick);
    };
  }, [addAccountDropdownOpen]);

  const cardStyle: React.CSSProperties = {
    backgroundColor: "var(--admiral-card)",
    border: "1px solid var(--admiral-border)",
    borderRadius: "var(--admiral-radius-card)",
    boxShadow: "var(--admiral-shadow-card)",
  };

  return (
    <div
      style={{
        maxWidth: 720,
        padding: 24,
        display: "flex",
        flexDirection: "column",
        gap: 24,
      }}
    >
      <h2
        style={{
          fontSize: 18,
          fontWeight: 600,
          color: "var(--admiral-text-primary)",
          margin: 0,
        }}
      >
        授权管理
      </h2>

      <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            padding: "6px 12px",
            borderRadius: "var(--admiral-radius-button)",
            border: "1px solid var(--admiral-border)",
            fontSize: 14,
            color: "var(--admiral-text-primary)",
          }}
        >
          [▼ B站]
        </div>
        <div style={{ flex: 1, minWidth: 180, display: "flex", gap: 12, flexWrap: "wrap" }}>
          <input
            type="text"
            placeholder="账户名称搜索"
            value={searchName}
            onChange={(e) => setSearchName(e.target.value)}
            style={{
              flex: 1,
              minWidth: 120,
              padding: "8px 12px",
              fontSize: 14,
              borderRadius: "var(--admiral-radius-input)",
              border: "1px solid var(--admiral-border)",
              color: "var(--admiral-text-primary)",
            }}
          />
          <input
            type="text"
            placeholder="账户ID精确匹配"
            value={searchId}
            onChange={(e) => setSearchId(e.target.value)}
            style={{
              width: 140,
              padding: "8px 12px",
              fontSize: 14,
              borderRadius: "var(--admiral-radius-input)",
              border: "1px solid var(--admiral-border)",
              color: "var(--admiral-text-primary)",
            }}
          />
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          <div style={{ position: "relative" }}>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setAddAccountDropdownOpen(!addAccountDropdownOpen);
              }}
              disabled={status.length === 0}
              className="btn btn-default"
            >
              新增账户 ▾
            </button>
            {addAccountDropdownOpen && status.length > 0 && (
              <div
                onClick={(e) => e.stopPropagation()}
                style={{
                  position: "absolute",
                  top: "100%",
                  left: 0,
                  marginTop: 4,
                  minWidth: 180,
                  backgroundColor: "var(--admiral-card)",
                  border: "1px solid var(--admiral-border)",
                  borderRadius: "var(--admiral-radius-button)",
                  boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
                  zIndex: 10,
                }}
              >
                {status.map((s) => (
                  <button
                    key={s.id}
                    type="button"
                    onClick={() => openBindModal(s.id)}
                    style={{
                      display: "block",
                      width: "100%",
                      padding: "10px 14px",
                      textAlign: "left",
                      fontSize: 14,
                      border: "none",
                      background: "none",
                      color: "var(--admiral-text-primary)",
                      cursor: "pointer",
                    }}
                  >
                    {s.user_name || (s.bid != null ? `BID: ${s.bid}` : s.id.slice(0, 8))}
                  </button>
                ))}
              </div>
            )}
          </div>
          <button type="button" onClick={handleInitiate} className="btn btn-default">
            新增授权
          </button>
        </div>
      </div>

      {error && (
        <div
          style={{
            padding: "10px 14px",
            fontSize: 14,
            color: "var(--color-danger)",
            backgroundColor: "rgba(245,108,108,0.1)",
            border: "1px solid var(--color-danger)",
            borderRadius: "var(--admiral-radius-input)",
          }}
        >
          {error}
        </div>
      )}

      <div style={cardStyle}>
        {loading ? (
          <div
            style={{
              padding: 32,
              textAlign: "center",
              color: "var(--admiral-text-secondary)",
              fontSize: 14,
            }}
          >
            加载中…
          </div>
        ) : filteredBindings.length > 0 ? (
          <>
            <div
              style={{
                padding: "16px 20px",
                borderBottom: "1px solid var(--admiral-border)",
                fontSize: 15,
                fontWeight: 600,
                color: "var(--admiral-text-primary)",
              }}
            >
              已绑定账户 ({filteredBindings.length})
            </div>
            <div style={{ overflowX: "auto" }}>
              <table
                style={{
                  width: "100%",
                  fontSize: 14,
                  borderCollapse: "collapse",
                }}
              >
                <thead>
                  <tr
                    style={{
                      borderBottom: "1px solid var(--admiral-border)",
                      color: "var(--admiral-text-secondary)",
                      textAlign: "left",
                    }}
                  >
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>账户名称</th>
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>账户ID</th>
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>操作</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredBindings.map((b) => (
                    <tr
                      key={`${b.platform}-${b.account_id}`}
                      style={{
                        borderBottom: "1px solid var(--admiral-border)",
                      }}
                    >
                      <td style={{ padding: "12px 16px" }}>{b.account_name ?? "—"}</td>
                      <td
                        style={{
                          padding: "12px 16px",
                          fontFamily: "monospace",
                        }}
                      >
                        {b.account_id}
                      </td>
                      <td style={{ padding: "12px 16px" }}>
                        <button
                          type="button"
                          onClick={() => setUnbindTarget(b)}
                          style={{
                            fontSize: 14,
                            color: "var(--color-danger)",
                            background: "none",
                            border: "none",
                            cursor: "pointer",
                            padding: "4px 8px",
                          }}
                        >
                          删除
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </>
        ) : (
          <div
            style={{
              padding: 40,
              textAlign: "center",
              color: "var(--admiral-text-secondary)",
              fontSize: 14,
            }}
          >
            <p style={{ margin: 0, marginBottom: 8 }}>暂无已绑定账户</p>
            <p style={{ margin: 0 }}>
              请先发起 B 站授权，完成授权后点击「新增账户」选择要绑定的广告账户。
            </p>
          </div>
        )}
      </div>

      <BindAccountModal
        open={addAccountOpen}
        onClose={() => {
          setAddAccountOpen(false);
          setSelectedTokenId(null);
        }}
        oauthSessionId={selectedTokenId}
        onSuccess={loadData}
      />

      <ConfirmUnbindDialog
        open={!!unbindTarget}
        accountName={unbindTarget?.account_name ?? ""}
        onClose={() => setUnbindTarget(null)}
        onConfirm={handleUnbind}
      />
    </div>
  );
}
