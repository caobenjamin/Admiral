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
    <div className="flex flex-col h-full max-w-2xl">
      <h2 className="text-lg font-medium mb-4">LLM 对话（联调）</h2>
      <div className="flex-1 overflow-auto rounded border p-4 bg-gray-50 space-y-2 mb-4">
        {messages.length === 0 && (
          <p className="text-gray-500 text-sm">发送一条消息开始对话</p>
        )}
        {messages.map((m, i) => (
          <div
            key={i}
            className={`rounded p-2 text-sm ${m.role === "user" ? "bg-blue-100 ml-8" : "bg-white mr-8"}`}
          >
            <span className="font-medium">{m.role === "user" ? "我" : "助手"}: </span>
            {m.content}
          </div>
        ))}
      </div>
      <div className="flex gap-2">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && handleSend()}
          placeholder="输入消息..."
          className="border rounded px-3 py-2 flex-1"
        />
        <button type="button" onClick={handleSend} disabled={loading} className="btn btn-primary">
          {loading ? "..." : "发送"}
        </button>
      </div>
    </div>
  );
}
