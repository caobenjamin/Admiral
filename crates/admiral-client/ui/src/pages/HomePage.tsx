import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatInput } from "@/components/chat/ChatInput";
import { MessageBubble, type ChatMessage } from "@/components/chat/MessageBubble";

export function HomePage() {
  const user = useAuthStore((s) => s.user);
  const { loadUserAndLicense } = useAuth();
  const navigate = useNavigate();
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  useEffect(() => {
    loadUserAndLicense().then((ok) => {
      if (!ok) navigate("/login");
    });
  }, [loadUserAndLicense, navigate]);

  if (!user) return null;

  const handleSend = (content: string) => {
    const userMsg: ChatMessage = {
      id: `u-${Date.now()}`,
      role: "user",
      content,
    };
    setMessages((prev) => [...prev, userMsg]);
    // 占位：后续接 Agent/LLM，这里追加一条模拟回复
    const assistantMsg: ChatMessage = {
      id: `a-${Date.now()}`,
      role: "assistant",
      content: "已收到您的指令。任务创建与 Agent 对话将在 Phase 3/4 接入。",
    };
    setTimeout(() => setMessages((prev) => [...prev, assistantMsg]), 400);
  };

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
        <div
          className="mt-4 rounded-lg border p-4"
          style={{
            borderColor: "var(--admiral-border)",
            backgroundColor: "var(--admiral-card)",
            borderRadius: "var(--admiral-radius-card)",
          }}
        >
          <h4 className="text-sm font-medium text-[var(--admiral-text-primary)] mb-3">Agent 对话</h4>
          {messages.length > 0 ? (
            <div className="space-y-2">
              {messages.map((m) => (
                <MessageBubble key={m.id} message={m} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-[var(--admiral-text-placeholder)] py-4">
              在下方输入自然语言描述任务，或与 Agent 对话
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
        }}
      >
        <ChatInput
          onSend={handleSend}
          placeholder="输入指令或描述任务…"
        />
      </div>
    </div>
  );
}
