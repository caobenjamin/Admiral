import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatPanel } from "@/components/chat/ChatPanel";
import type { ChatMessage } from "@/components/chat/MessageBubble";

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
        gap: 16,
      }}
    >
      <div style={{ flexShrink: 0, minHeight: 0, overflow: "auto" }}>
        <TaskDetail />
      </div>
      <div
        style={{
          flex: 1,
          minHeight: 200,
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        <ChatPanel
          messages={messages}
          onSend={handleSend}
          placeholder="输入指令或描述任务…"
        />
      </div>
    </div>
  );
}
