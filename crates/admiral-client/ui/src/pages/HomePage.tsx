import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { useAgent } from "@/hooks/useAgent";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatInput } from "@/components/chat/ChatInput";
import { MessageBubble, type ChatMessage } from "@/components/chat/MessageBubble";

function sessionMessagesToChatMessages(
  sessionMessages: { role: string; content?: string | null; tool_calls?: unknown }[]
): ChatMessage[] {
  return sessionMessages.map((m, i) => {
    let content = m.content ?? "";
    if (m.role === "assistant" && m.tool_calls && Array.isArray(m.tool_calls) && m.tool_calls.length > 0) {
      if (!content) content = "[调用工具]";
    }

    const role =
      m.role === "user"
        ? "user"
        : m.role === "assistant"
          ? "assistant"
          : m.role === "system"
            ? "system"
            : "tool";

    return { id: `msg-${i}`, role, content };
  });
}

const QUICK_ACTIONS = ["查询今日投放概览", "继续执行当前任务", "导出本次结果摘要"];

export function HomePage() {
  const user = useAuthStore((s) => s.user);
  const { loadUserAndLicense } = useAuth();
  const { chat, loading, error } = useAgent(null);
  const navigate = useNavigate();
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  useEffect(() => {
    loadUserAndLicense().then((ok) => {
      if (!ok) navigate("/login");
    });
  }, [loadUserAndLicense, navigate]);

  const handleSend = useCallback(
    async (content: string) => {
      const userMsg: ChatMessage = { id: `u-${Date.now()}`, role: "user", content };
      setMessages((prev) => [...prev, userMsg]);
      try {
        const response = await chat(content);
        const list = sessionMessagesToChatMessages(response.session.messages);
        setMessages(list);
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : "请求失败，请重试。";
        setMessages((prev) => [...prev, { id: `a-${Date.now()}`, role: "assistant", content: errMsg }]);
      }
    },
    [chat]
  );

  if (!user) return null;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        minHeight: 0,
        overflow: "hidden",
        padding: 14,
        gap: 0,
      }}
    >
      <div style={{ flex: 1, minHeight: 0, overflow: "auto", paddingBottom: 12 }}>
        <TaskDetail />

        <div className="admiral-surface" style={{ marginTop: 12, padding: 12 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, marginBottom: 8 }}>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>Agent 对话</h3>
              <p style={{ fontSize: 12, color: "var(--admiral-text-secondary)", margin: "3px 0 0" }}>
                摘要优先展示，工具结果可按需展开
              </p>
            </div>
            <span className="admiral-chip admiral-chip-primary">实时会话</span>
          </div>

          <div
            style={{
              border: "1px solid var(--admiral-border)",
              borderRadius: 12,
              backgroundColor: "#fbfdff",
              minHeight: 130,
              maxHeight: 430,
              overflowY: "auto",
              padding: "10px 12px",
            }}
          >
            {messages.length > 0 ? (
              messages.map((m) => <MessageBubble key={m.id} message={m} />)
            ) : (
              <p style={{ margin: "28px 0", fontSize: 13, color: "var(--admiral-text-placeholder)", textAlign: "center" }}>
                暂无对话。在下方输入框发送消息开始。
              </p>
            )}
          </div>

          {error && (
            <div className="admiral-chip admiral-chip-danger" style={{ marginTop: 8 }}>
              {error}
            </div>
          )}
        </div>
      </div>

      <div
        style={{
          flexShrink: 0,
          borderTop: "1px solid var(--admiral-border)",
          paddingTop: 12,
          backgroundColor: "transparent",
        }}
      >
        <ChatInput
          onSend={handleSend}
          placeholder="输入指令或描述任务..."
          disabled={loading}
          loading={loading}
          quickActions={QUICK_ACTIONS}
        />
      </div>
    </div>
  );
}
