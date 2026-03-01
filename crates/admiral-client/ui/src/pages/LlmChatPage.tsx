import { useState } from "react";
import * as tauri from "@/lib/tauri";

export function LlmChatPage() {
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<{ role: string; content: string }[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSend = async () => {
    if (!input.trim()) return;
    const userMessage = input.trim();
    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: userMessage }]);
    setLoading(true);
    try {
      const msgs = [
        ...messages.map((m) => ({ role: m.role, content: m.content })),
        { role: "user" as const, content: userMessage },
      ];
      const bytes = await tauri.llmChat(msgs, false);
      const text = new TextDecoder().decode(new Uint8Array(bytes));
      let content = "";
      try {
        const json = JSON.parse(text);
        content = json.choices?.[0]?.message?.content ?? json.content ?? text;
      } catch {
        content = text;
      }
      setMessages((prev) => [...prev, { role: "assistant", content }]);
    } catch (e) {
      setMessages((prev) => [...prev, { role: "assistant", content: `错误: ${e}` }]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 900, display: "flex", flexDirection: "column", minHeight: 540 }}>
        <h2 className="settings-title">LLM 对话（联调）</h2>
        <p className="settings-subtitle">用于验证模型代理返回结构与文本质量。</p>

        <div
          style={{
            marginTop: 14,
            flex: 1,
            overflow: "auto",
            border: "1px solid var(--admiral-border)",
            borderRadius: 12,
            padding: 10,
            background: "#f8fbff",
            display: "flex",
            flexDirection: "column",
            gap: 8,
            minHeight: 320,
          }}
        >
          {messages.length === 0 && <p style={{ margin: "auto", fontSize: 13, color: "var(--admiral-text-placeholder)" }}>发送一条消息开始对话</p>}
          {messages.map((m, i) => (
            <div
              key={i}
              style={{
                borderRadius: 12,
                padding: "8px 10px",
                fontSize: 13,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                alignSelf: m.role === "user" ? "flex-end" : "flex-start",
                maxWidth: "88%",
                background: m.role === "user" ? "linear-gradient(135deg, #0f6cbd 0%, #0b5ca4 100%)" : "#ffffff",
                border: m.role === "user" ? "1px solid #0f6cbd" : "1px solid var(--admiral-border)",
                color: m.role === "user" ? "#ffffff" : "var(--admiral-text-primary)",
              }}
            >
              <span style={{ fontWeight: 700, fontSize: 12, marginRight: 6 }}>{m.role === "user" ? "我" : "助手"}</span>
              {m.content}
            </div>
          ))}
        </div>

        <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && handleSend()}
            placeholder="输入消息..."
            className="admiral-field"
            style={{ padding: "0 10px", height: 36, flex: 1, outline: "none" }}
          />
          <button type="button" onClick={handleSend} disabled={loading} className="btn btn-primary" style={{ minWidth: 78 }}>
            {loading ? "发送中" : "发送"}
          </button>
        </div>
      </div>
    </div>
  );
}
