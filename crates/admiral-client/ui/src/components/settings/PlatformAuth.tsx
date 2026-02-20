import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import type {
  TokenMeta,
  OAuthAccountsResponse,
  BoundAccountRecord,
  BiliAdvertiserRecord,
} from "@/lib/types";

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

/** 账户绑定弹窗 */
function BindAccountModal({
  open,
  onClose,
  status,
  onSuccess,
}: {
  open: boolean;
  onClose: () => void;
  status: TokenMeta[];
  onSuccess: () => void;
}) {
  const [accounts, setAccounts] = useState<OAuthAccountsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  const [bindLoading, setBindLoading] = useState(false);

  const loadAccounts = useCallback(async () => {
    if (status.length === 0) return;
    setLoading(true);
    setError("");
    try {
      const data = await tauri.oauthAccountsList();
      setAccounts(data);
      if (status.length > 0) {
        setSelectedAccountIds([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [status.length]);

  useEffect(() => {
    if (open) {
      loadAccounts();
    } else {
      setAccounts(null);
      setError("");
      setSelectedAccountIds([]);
    }
  }, [open, loadAccounts]);

  const handleBind = async () => {
    const tokenId = status[0]?.id;
    if (!tokenId || selectedAccountIds.length === 0) return;
    setBindLoading(true);
    setError("");
    try {
      await tauri.oauthBindAccounts(tokenId, selectedAccountIds);
      onSuccess();
      onClose();
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
        backgroundColor: "rgba(0,0,0,0.4)",
      }}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
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
          账户绑定
        </div>
        <div style={{ padding: 20, overflow: "auto", flex: 1 }}>
          {status.length === 0 ? (
            <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14 }}>
              请先发起 B 站授权，完成授权后再绑定账户。
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
                共 {accounts.total} 个账户，勾选需要绑定的账户后点击确认。
              </p>
              <div style={{ marginBottom: 12, display: "flex", gap: 12 }}>
                <button
                  type="button"
                  onClick={selectAll}
                  className="text-sm hover:underline"
                  style={{ color: "var(--color-primary)" }}
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
                      <th style={{ padding: "10px 12px 10px 0", width: 44 }}>
                        选择
                      </th>
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
                        <td style={{ padding: "10px 0" }}>
                          {r.account_role ?? "—"}
                        </td>
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
          <button
            type="button"
            onClick={onClose}
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
            取消
          </button>
          {status.length > 0 &&
            accounts &&
            accounts.records.length > 0 && (
              <button
                type="button"
                onClick={handleBind}
                disabled={bindLoading || selectedAccountIds.length === 0}
                style={{
                  padding: "8px 16px",
                  fontSize: 14,
                  fontWeight: 500,
                  borderRadius: "var(--admiral-radius-button)",
                  border: "none",
                  backgroundColor:
                    bindLoading || selectedAccountIds.length === 0
                      ? "var(--admiral-border)"
                      : "var(--color-primary)",
                  color: "#fff",
                  cursor:
                    bindLoading || selectedAccountIds.length === 0
                      ? "not-allowed"
                      : "pointer",
                }}
              >
                {bindLoading ? "提交中…" : `确认绑定 (${selectedAccountIds.length})`}
              </button>
            )}
        </div>
      </div>
    </div>
  );
}

export function PlatformAuth() {
  const [status, setStatus] = useState<TokenMeta[]>([]);
  const [bindings, setBindings] = useState<BoundAccountRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [modalOpen, setModalOpen] = useState(false);

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

  const handleInitiate = async () => {
    setError("");
    try {
      const { authorization_url } = await tauri.oauthInitiate();
      await invoke("open_url", { url: authorization_url });
      setTimeout(loadData, 2000);
    } catch (e) {
      setError(String(e));
    }
  };

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
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", flexWrap: "wrap", gap: 16 }}>
        <h2
          style={{
            fontSize: 18,
            fontWeight: 600,
            color: "var(--admiral-text-primary)",
            margin: 0,
          }}
        >
          B 站 媒体账户
        </h2>
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <button
            type="button"
            onClick={() => setModalOpen(true)}
            style={{
              padding: "8px 16px",
              fontSize: 14,
              fontWeight: 500,
              borderRadius: "var(--admiral-radius-button)",
              border: "none",
              backgroundColor: "var(--color-primary)",
              color: "#fff",
              cursor: "pointer",
            }}
          >
            账户绑定
          </button>
          <button
            type="button"
            onClick={handleInitiate}
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
            发起授权
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
        ) : bindings.length > 0 ? (
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
              已绑定账户 ({bindings.length})
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
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>
                      平台
                    </th>
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>
                      账户 ID
                    </th>
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>
                      账户名称
                    </th>
                    <th style={{ padding: "12px 16px", fontWeight: 500 }}>
                      绑定时间
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {bindings.map((b) => (
                    <tr
                      key={`${b.platform}-${b.account_id}`}
                      style={{
                        borderBottom: "1px solid var(--admiral-border)",
                      }}
                    >
                      <td style={{ padding: "12px 16px" }}>{b.platform}</td>
                      <td
                        style={{
                          padding: "12px 16px",
                          fontFamily: "monospace",
                        }}
                      >
                        {b.account_id}
                      </td>
                      <td style={{ padding: "12px 16px" }}>
                        {b.account_name ?? "—"}
                      </td>
                      <td
                        style={{
                          padding: "12px 16px",
                          color: "var(--admiral-text-secondary)",
                        }}
                      >
                        {formatDate(b.created_at)}
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
            <p style={{ margin: 0, marginBottom: 8 }}>
              暂无已绑定账户。
            </p>
            <p style={{ margin: 0 }}>
              请先发起 B 站授权，完成授权后点击「账户绑定」选择要绑定的广告账户。
            </p>
          </div>
        )}
      </div>

      {/* 授权会话管理（折叠或次要展示） */}
      {status.length > 0 && (
        <details
          style={{
            ...cardStyle,
            overflow: "hidden",
          }}
        >
          <summary
            style={{
              padding: "12px 16px",
              fontSize: 14,
              fontWeight: 500,
              color: "var(--admiral-text-secondary)",
              cursor: "pointer",
            }}
          >
            授权会话 ({status.length}) — 点击展开管理
          </summary>
          <div
            style={{
              borderTop: "1px solid var(--admiral-border)",
              padding: 12,
            }}
          >
            {status.map((s) => (
              <div
                key={s.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "8px 12px",
                  fontSize: 13,
                  color: "var(--admiral-text-primary)",
                  borderBottom: "1px solid var(--admiral-border)",
                }}
              >
                <span>
                  {s.platform} · {formatDate(s.created_at)}
                </span>
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      await tauri.oauthDeleteSession(s.id);
                      loadData();
                    } catch (e) {
                      setError(String(e));
                    }
                  }}
                  style={{
                    fontSize: 12,
                    color: "var(--color-danger)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                    padding: "4px 8px",
                  }}
                >
                  取消授权
                </button>
              </div>
            ))}
          </div>
        </details>
      )}

      <BindAccountModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        status={status}
        onSuccess={loadData}
      />
    </div>
  );
}
