import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as tauri from "@/lib/tauri";
import { useToastStore } from "@/stores/toastStore";
import type {
  TokenMeta,
  BiliSubjectRecord,
  OAuthAccountsResponse,
  BoundAccountRecord,
  BiliAdvertiserRecord,
} from "@/lib/types";

const cardStyle: React.CSSProperties = {
  backgroundColor: "var(--admiral-card)",
  border: "1px solid var(--admiral-border)",
  borderRadius: "var(--admiral-radius-card)",
  boxShadow: "var(--admiral-shadow-card)",
};

function ErrorNotice({ message }: { message: string }) {
  if (!message) return null;

  return <div className="oauth-error-notice">{message}</div>;
}

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
      className="oauth-overlay"
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="admiral-dialog-content oauth-modal"
        style={{
          width: "min(92vw, 460px)",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="oauth-modal-header" id="unbind-dialog-title">
          <span className="oauth-modal-title">确认解绑</span>
          <span className="admiral-chip admiral-chip-danger">危险操作</span>
        </div>

        <div style={{ padding: "14px 16px", color: "var(--admiral-text-primary)", fontSize: 14 }}>
          确定要解绑该账户{accountName ? `「${accountName}」` : ""}吗？解绑后将无法继续执行该账户相关任务。
        </div>

        <div
          className="oauth-modal-footer"
          style={{ borderTop: "none", paddingTop: 10 }}
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
  const [subjects, setSubjects] = useState<BiliSubjectRecord[]>([]);
  const [subjectsLoading, setSubjectsLoading] = useState(false);
  const [selectedSubject, setSelectedSubject] = useState<BiliSubjectRecord | null>(null);
  const [accounts, setAccounts] = useState<OAuthAccountsResponse | null>(null);
  const [accountsLoading, setAccountsLoading] = useState(false);
  const [error, setError] = useState("");
  const [selectedAccountIds, setSelectedAccountIds] = useState<number[]>([]);
  const [bindLoading, setBindLoading] = useState(false);

  const loadSubjects = useCallback(async () => {
    if (!oauthSessionId) return;
    setSubjectsLoading(true);
    setError("");
    try {
      const list = await tauri.oauthSubjectsList(oauthSessionId);
      setSubjects(list);
      setSelectedSubject(null);
      setAccounts(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setSubjectsLoading(false);
    }
  }, [oauthSessionId]);

  const loadAccounts = useCallback(async () => {
    if (!oauthSessionId || !selectedSubject) return;
    setAccountsLoading(true);
    setError("");
    try {
      const data = await tauri.oauthAccountsList(
        oauthSessionId,
        selectedSubject.subject_type,
        selectedSubject.subject_id
      );
      setAccounts(data);
      setSelectedAccountIds([]);
    } catch (e) {
      setError(String(e));
    } finally {
      setAccountsLoading(false);
    }
  }, [oauthSessionId, selectedSubject]);

  useEffect(() => {
    if (open && oauthSessionId) {
      loadSubjects();
    } else {
      setSubjects([]);
      setSelectedSubject(null);
      setAccounts(null);
      setError("");
      setSelectedAccountIds([]);
    }
  }, [open, oauthSessionId, loadSubjects]);

  useEffect(() => {
    if (open && oauthSessionId && selectedSubject) {
      loadAccounts();
    }
  }, [open, oauthSessionId, selectedSubject?.subject_type, selectedSubject?.subject_id, loadAccounts]);

  const handleBind = async () => {
    if (!oauthSessionId || !selectedSubject || selectedAccountIds.length === 0 || !accounts) return;
    const accountNames = selectedAccountIds.map(
      (id) => accounts.records.find((r) => r.account_id === id)?.account_name ?? ""
    );
    setBindLoading(true);
    setError("");
    try {
      await tauri.oauthBindAccounts(
        oauthSessionId,
        selectedSubject.subject_type,
        selectedSubject.subject_id,
        selectedAccountIds,
        accountNames
      );
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

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="bind-modal-title"
      className="oauth-overlay"
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="admiral-dialog-content oauth-modal"
        style={{
          width: "min(92vw, 620px)",
          maxHeight: "82vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="oauth-modal-header" id="bind-modal-title">
          <span className="oauth-modal-title">新增账户</span>
          <span className="admiral-chip admiral-chip-primary">Step 1 选择组织 · Step 2 绑定账户</span>
        </div>

        <div className="oauth-modal-body">
          {!oauthSessionId ? (
            <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14, margin: 0 }}>
              请先选择授权来源（BID / 用户名）。
            </p>
          ) : subjectsLoading ? (
            <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14, margin: 0 }}>
              正在获取组织列表...
            </p>
          ) : !selectedSubject ? (
            <>
              <p style={{ fontSize: 13, color: "var(--admiral-text-secondary)", margin: 0 }}>
                请先选择一个组织（subject），再拉取该组织下的广告主账户。
              </p>
              {subjects.length > 0 ? (
                <div style={{ display: "grid", gap: 8 }}>
                  {subjects.map((s) => (
                    <button
                      key={`${s.subject_type}-${s.subject_id}`}
                      type="button"
                      onClick={() => setSelectedSubject(s)}
                      style={{
                        padding: "10px 12px",
                        textAlign: "left",
                        fontSize: 13,
                        border: "1px solid var(--admiral-border)",
                        borderRadius: "var(--admiral-radius-input)",
                        backgroundColor: "#ffffff",
                        color: "var(--admiral-text-primary)",
                        cursor: "pointer",
                      }}
                    >
                      组织 type={s.subject_type} id={s.subject_id}
                    </button>
                  ))}
                </div>
              ) : (
                <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14, margin: 0 }}>
                  当前授权下无可用组织。
                </p>
              )}
            </>
          ) : (
            <>
              <ErrorNotice message={error} />

              {!error && accountsLoading && (
                <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14, margin: 0 }}>
                  正在获取可绑定账户列表...
                </p>
              )}

              {!error && accounts && accounts.records.length > 0 && (
                <>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <p style={{ fontSize: 13, color: "var(--admiral-text-secondary)", margin: 0 }}>
                      当前组织：type={selectedSubject.subject_type} id={selectedSubject.subject_id}
                    </p>
                    <button
                      type="button"
                      onClick={() => {
                        setSelectedSubject(null);
                        setAccounts(null);
                        setError("");
                      }}
                      className="btn btn-default btn-small"
                    >
                      更换组织
                    </button>
                  </div>

                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8, flexWrap: "wrap" }}>
                    <p style={{ fontSize: 13, color: "var(--admiral-text-secondary)", margin: 0 }}>
                      共 {accounts.total} 个账户，勾选需要绑定的账户后点击保存。
                    </p>
                    <div style={{ display: "flex", gap: 8 }}>
                      <button type="button" onClick={selectAll} className="btn btn-default btn-small">全选</button>
                      <button type="button" onClick={clearSelection} className="btn btn-default btn-small">清空</button>
                    </div>
                  </div>

                  <div style={{ overflowX: "auto", border: "1px solid var(--admiral-border)", borderRadius: 12, background: "#fafdff" }}>
                    <table className="oauth-table" style={{ fontSize: 13 }}>
                      <thead>
                        <tr>
                          <th style={{ width: 52 }}>选择</th>
                          <th>账户 ID</th>
                          <th>账户名称</th>
                          <th>角色</th>
                        </tr>
                      </thead>
                      <tbody>
                        {accounts.records.map((r: BiliAdvertiserRecord) => (
                          <tr key={r.account_id} style={{ borderBottom: "1px solid var(--admiral-border)" }}>
                            <td>
                              <input
                                type="checkbox"
                                checked={selectedAccountIds.includes(r.account_id)}
                                onChange={() => toggleAccount(r.account_id)}
                                style={{ width: 16, height: 16 }}
                              />
                            </td>
                            <td style={{ fontFamily: "var(--admiral-font-mono)" }}>
                              {r.account_id}
                            </td>
                            <td>{r.account_name ?? "-"}</td>
                            <td>{r.account_role ?? "-"}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}

              {!error && accounts && accounts.records.length === 0 && (
                <p style={{ color: "var(--admiral-text-secondary)", fontSize: 14, margin: 0 }}>
                  当前授权下无可绑定账户。
                </p>
              )}
            </>
          )}
        </div>

        <div className="oauth-modal-footer">
          <button type="button" onClick={onClose} className="btn btn-default">
            取消
          </button>
          {oauthSessionId && accounts && accounts.records.length > 0 && (
            <button
              type="button"
              onClick={handleBind}
              disabled={bindLoading || selectedAccountIds.length === 0}
              className="btn btn-primary"
            >
              {bindLoading ? "保存中..." : `保存 (${selectedAccountIds.length})`}
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
      list = list.filter((b) => (b.account_name ?? "").toLowerCase().includes(kw));
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

  useEffect(() => {
    if (!addAccountDropdownOpen) return;
    const onDocClick = () => setAddAccountDropdownOpen(false);
    const t = setTimeout(() => document.addEventListener("click", onDocClick), 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener("click", onDocClick);
    };
  }, [addAccountDropdownOpen]);

  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 980, display: "flex", flexDirection: "column", gap: 14 }}>
        <h2 className="settings-title">授权管理</h2>
        <p className="settings-subtitle">管理 B 站授权、组织下账户绑定与解绑。</p>

        <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
          <div className="admiral-chip admiral-chip-primary">B站</div>

          <div style={{ flex: 1, minWidth: 180, display: "flex", gap: 10, flexWrap: "wrap" }}>
            <input
              type="text"
              placeholder="账户名称搜索"
              value={searchName}
              onChange={(e) => setSearchName(e.target.value)}
              className="admiral-field oauth-field"
              style={{ flex: 1, minWidth: 150 }}
            />
            <input
              type="text"
              placeholder="账户ID精确匹配"
              value={searchId}
              onChange={(e) => setSearchId(e.target.value)}
              className="admiral-field oauth-field"
              style={{ width: 150 }}
            />
          </div>

          <div style={{ display: "flex", gap: 8 }}>
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
                  className="oauth-dropdown"
                >
                  {status.map((s) => (
                    <button
                      key={s.id}
                      type="button"
                      onClick={() => openBindModal(s.id)}
                      className="oauth-dropdown-item header-dropdown-item"
                    >
                      {s.user_name || (s.bid != null ? `BID: ${s.bid}` : s.id.slice(0, 8))}
                    </button>
                  ))}
                </div>
              )}
            </div>

            <button type="button" onClick={handleInitiate} className="btn btn-default">
              {initiatePolling ? "授权中..." : "新增授权"}
            </button>
          </div>
        </div>

        <ErrorNotice message={error} />

        <div style={cardStyle}>
          {loading ? (
            <div className="oauth-loading">加载中...</div>
          ) : filteredBindings.length > 0 ? (
            <>
              <div
                style={{
                  padding: "14px 16px",
                  borderBottom: "1px solid var(--admiral-border)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 8,
                  flexWrap: "wrap",
                }}
              >
                <span style={{ fontSize: 15, fontWeight: 700, color: "var(--admiral-text-primary)" }}>
                  已绑定账户 ({filteredBindings.length})
                </span>
                <span className="admiral-chip">仅展示当前筛选结果</span>
              </div>

              <div className="oauth-table-wrap">
                <table className="oauth-table">
                  <thead>
                    <tr>
                      <th>账户名称</th>
                      <th>账户 ID</th>
                      <th>操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredBindings.map((b) => (
                      <tr key={`${b.platform}-${b.account_id}`}>
                        <td>{b.account_name ?? "-"}</td>
                        <td style={{ fontFamily: "var(--admiral-font-mono)" }}>
                          {b.account_id}
                        </td>
                        <td>
                          <button
                            type="button"
                            onClick={() => setUnbindTarget(b)}
                            className="btn btn-danger btn-small"
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
            <div className="oauth-empty">
              <p style={{ margin: 0, marginBottom: 6 }}>暂无已绑定账户</p>
              <p style={{ margin: 0 }}>请先发起 B 站授权，再点击「新增账户」选择要绑定的广告账户。</p>
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
    </div>
  );
}
