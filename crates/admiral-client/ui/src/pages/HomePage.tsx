import { useEffect, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { useAgent } from "@/hooks/useAgent";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatInput } from "@/components/chat/ChatInput";
import { MessageBubble, type ChatMessage } from "@/components/chat/MessageBubble";

function sessionMessagesToChatMessages(
  sessionMessages: { role: string; content?: string | null; tool_calls?: unknown; name?: string | null }[]
): ChatMessage[] {
  return sessionMessages.map((m, i) => {
    let content = m.content ?? "";
    if (m.role === "assistant" && m.tool_calls && Array.isArray(m.tool_calls) && m.tool_calls.length > 0) {
      if (!content) content = "[调用工具]";
    }
    const role = m.role === "user" ? "user" : m.role === "assistant" ? "assistant" : m.role === "system" ? "system" : "tool";
    return { id: `msg-${i}`, role, content };
  });
}

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
        setMessages((prev) => [
          ...prev,
          { id: `a-${Date.now()}`, role: "assistant", content: errMsg },
        ]);
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
        padding: 16,
        gap: 0,
      }}
    >
      {/* 展示区域（可滚动）：任务详情、执行时间线、Agent 对话 */}
      <div
        style={{
          flex: 1,
          minHeight: 0,
          overflow: "auto",
          paddingBottom: 16,
        }}
      >
        <TaskDetail />
        {/* Agent 对话：独立卡片区块 */}
        <div
          className="mt-4 rounded-lg border p-4"
          style={{
            borderColor: "var(--admiral-border)",
            backgroundColor: "var(--admiral-card)",
            borderRadius: "var(--admiral-radius-card)",
            boxShadow: "var(--admiral-shadow-card)",
          }}
        >
          <h3
            className="text-[var(--admiral-text-primary)]"
            style={{ fontSize: 14, fontWeight: 600, margin: "0 0 4px 0" }}
          >
            Agent 对话
          </h3>
          <p className="text-xs text-[var(--admiral-text-secondary)] mb-3" style={{ marginTop: 4 }}>
            在下方输入自然语言描述任务，或与 Agent 对话
          </p>
          {messages.length > 0 ? (
            <div className="space-y-2">
              {messages.map((m) => (
                <MessageBubble key={m.id} message={m} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-[var(--admiral-text-placeholder)] py-4">
              暂无对话，在下方输入框发送消息开始
            </p>
          )}
        </div>
      </div>
      {/* 输入区域（固定底部） */}
      <div
        style={{
          flexShrink: 0,
          borderTop: "1px solid var(--admiral-border)",
          paddingTop: 16,
          backgroundColor: "var(--admiral-card)",
        }}
      >
        <ChatInput
          onSend={handleSend}
          placeholder="输入指令或描述任务…"
          disabled={loading}
          loading={loading}
        />
      </div>
    </div>
  );
}
