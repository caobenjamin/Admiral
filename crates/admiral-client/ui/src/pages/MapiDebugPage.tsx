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
    <div className="space-y-4">
      <h2 className="text-lg font-medium">MAPI 调试</h2>
      <div className="grid gap-2 max-w-xl">
        <label className="flex gap-2 items-center">
          <span className="w-24">account_id</span>
          <input
            type="text"
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
            className="border rounded px-3 py-2 flex-1"
          />
        </label>
        <label className="flex gap-2 items-center">
          <span className="w-24">method</span>
          <select
            value={method}
            onChange={(e) => setMethod(e.target.value)}
            className="border rounded px-3 py-2"
          >
            <option value="GET">GET</option>
            <option value="POST">POST</option>
          </select>
        </label>
        <label className="flex gap-2 items-center">
          <span className="w-24">path</span>
          <input
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            className="border rounded px-3 py-2 flex-1"
          />
        </label>
        <label className="flex gap-2">
          <span className="w-24 shrink-0">query (JSON)</span>
          <textarea
            value={queryJson}
            onChange={(e) => setQueryJson(e.target.value)}
            className="border rounded px-3 py-2 flex-1 font-mono text-sm"
            rows={2}
          />
        </label>
        <label className="flex gap-2">
          <span className="w-24 shrink-0">body (JSON)</span>
          <textarea
            value={bodyJson}
            onChange={(e) => setBodyJson(e.target.value)}
            className="border rounded px-3 py-2 flex-1 font-mono text-sm"
            rows={4}
            placeholder="POST 时填写"
          />
        </label>
        <button type="button" onClick={handleSend} disabled={loading} className="btn btn-primary">
          {loading ? "请求中..." : "发送"}
        </button>
      </div>
      {result && (
        <pre className="rounded border p-4 bg-gray-50 text-sm overflow-auto max-h-96 whitespace-pre-wrap">
          {result}
        </pre>
      )}
    </div>
  );
}
