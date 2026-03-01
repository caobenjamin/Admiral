import { useState } from "react";
import * as tauri from "@/lib/tauri";

export function MapiDebugPage() {
  const [accountId, setAccountId] = useState("");
  const [method, setMethod] = useState("GET");
  const [path, setPath] = useState("v2/campaign/list");
  const [queryJson, setQueryJson] = useState("{}");
  const [bodyJson, setBodyJson] = useState("");
  const [result, setResult] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSend = async () => {
    setLoading(true);
    setResult("");
    try {
      let query: Record<string, unknown> | undefined;
      try {
        query = JSON.parse(queryJson || "{}");
      } catch {
        setResult("query 不是合法 JSON");
        return;
      }
      let body: Record<string, unknown> | undefined;
      if (bodyJson.trim()) {
        try {
          body = JSON.parse(bodyJson);
        } catch {
          setResult("body 不是合法 JSON");
          return;
        }
      }
      const [status, bytes] = await tauri.mapiProxy({
        account_id: Number(accountId),
        method,
        path,
        query: query && Object.keys(query).length > 0 ? query : undefined,
        body,
      });
      const text = new TextDecoder().decode(new Uint8Array(bytes));
      setResult(`Status: ${status}\n\n${text}`);
    } catch (e) {
      setResult(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 900 }}>
        <h2 className="settings-title">MAPI 调试</h2>
        <p className="settings-subtitle">用于联调代理请求。正式环境建议仅管理员可见。</p>

        <div style={{ marginTop: 14, display: "grid", gap: 10, maxWidth: 760 }}>
          <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ width: 96, color: "var(--admiral-text-secondary)", fontSize: 13 }}>account_id</span>
            <input
              type="text"
              value={accountId}
              onChange={(e) => setAccountId(e.target.value)}
              className="admiral-field"
              style={{ padding: "8px 10px", flex: 1, outline: "none" }}
            />
          </label>

          <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ width: 96, color: "var(--admiral-text-secondary)", fontSize: 13 }}>method</span>
            <select
              value={method}
              onChange={(e) => setMethod(e.target.value)}
              className="admiral-field"
              style={{ padding: "8px 10px", outline: "none" }}
            >
              <option value="GET">GET</option>
              <option value="POST">POST</option>
            </select>
          </label>

          <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <span style={{ width: 96, color: "var(--admiral-text-secondary)", fontSize: 13 }}>path</span>
            <input
              type="text"
              value={path}
              onChange={(e) => setPath(e.target.value)}
              className="admiral-field"
              style={{ padding: "8px 10px", flex: 1, outline: "none" }}
            />
          </label>

          <label style={{ display: "flex", gap: 8 }}>
            <span style={{ width: 96, flexShrink: 0, color: "var(--admiral-text-secondary)", fontSize: 13 }}>query (JSON)</span>
            <textarea
              value={queryJson}
              onChange={(e) => setQueryJson(e.target.value)}
              className="admiral-field"
              style={{ padding: "8px 10px", flex: 1, fontFamily: "var(--admiral-font-mono)", fontSize: 13, outline: "none" }}
              rows={2}
            />
          </label>

          <label style={{ display: "flex", gap: 8 }}>
            <span style={{ width: 96, flexShrink: 0, color: "var(--admiral-text-secondary)", fontSize: 13 }}>body (JSON)</span>
            <textarea
              value={bodyJson}
              onChange={(e) => setBodyJson(e.target.value)}
              className="admiral-field"
              style={{ padding: "8px 10px", flex: 1, fontFamily: "var(--admiral-font-mono)", fontSize: 13, outline: "none" }}
              rows={4}
              placeholder="POST 时填写"
            />
          </label>

          <button type="button" onClick={handleSend} disabled={loading} className="btn btn-primary" style={{ width: 120 }}>
            {loading ? "请求中..." : "发送"}
          </button>
        </div>

        {result && (
          <pre
            className="admiral-field"
            style={{
              marginTop: 14,
              padding: 12,
              background: "#f8fbff",
              fontSize: 12,
              overflow: "auto",
              maxHeight: 420,
              whiteSpace: "pre-wrap",
              fontFamily: "var(--admiral-font-mono)",
            }}
          >
            {result}
          </pre>
        )}
      </div>
    </div>
  );
}
