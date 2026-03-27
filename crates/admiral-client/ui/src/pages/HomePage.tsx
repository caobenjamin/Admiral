import { useEffect, useState, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";
import { useAgent } from "@/hooks/useAgent";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore } from "@/stores/taskStore";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatInput } from "@/components/chat/ChatInput";
import { MessageBubble, type ChatMessage } from "@/components/chat/MessageBubble";
import * as tauri from "@/lib/tauri";

function sessionMessagesToChatMessages(
  sessionMessages: { role: string; content?: string | null; tool_calls?: unknown }[]
): ChatMessage[] {
  return sessionMessages
    .filter((m) => m.role === "user" || m.role === "assistant")
    .map((m, i) => {
      let content = m.content ?? "";
      if (m.role === "assistant" && m.tool_calls && Array.isArray(m.tool_calls) && m.tool_calls.length > 0) {
        if (!content) content = "[调用工具]";
      }
      return {
        id: `msg-${i}`,
        role: m.role as "user" | "assistant",
        content,
      };
    });
}

const QUICK_ACTIONS = ["查询今日投放概览", "继续执行当前任务", "导出本次结果摘要"];

export function HomePage() {
  const user = useAuthStore((s) => s.user);
  const { loadUserAndLicense } = useAuth();
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const { fetchTasks } = useTaskStore();
  const navigate = useNavigate();

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [showTaskDetail, setShowTaskDetail] = useState(true);
  const prevTaskIdRef = useRef<string | null>(null);
  const streamingMsgIdRef = useRef<string | null>(null);

  const { chat, loading, error } = useAgent(selectedTaskId, {
    onDelta(content) {
      const msgId = streamingMsgIdRef.current;
      if (!msgId) return;
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.id === msgId) {
          return [...prev.slice(0, -1), { ...last, content: last.content + content }];
        }
        return prev;
      });
    },
    onToolUse(name) {
      const msgId = streamingMsgIdRef.current;
      if (!msgId) return;
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.id === msgId) {
          const toolHint = `\n\n> 调用工具: ${name} ...`;
          return [...prev.slice(0, -1), { ...last, content: last.content + toolHint }];
        }
        return prev;
      });
    },
    onToolResult(name, success) {
      const msgId = streamingMsgIdRef.current;
      if (!msgId) return;
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.id === msgId) {
          const resultHint = success ? ` 完成` : ` 失败`;
          return [...prev.slice(0, -1), { ...last, content: last.content + resultHint + "\n\n" }];
        }
        return prev;
      });
    },
  });

  useEffect(() => {
    loadUserAndLicense().then((ok) => {
      if (!ok) navigate("/login");
    });
  }, [loadUserAndLicense, navigate]);

  useEffect(() => {
    if (selectedTaskId === prevTaskIdRef.current) return;
    prevTaskIdRef.current = selectedTaskId;

    if (!selectedTaskId) {
      setMessages([]);
      return;
    }

    tauri.taskLoadConversations(selectedTaskId).then((rows) => {
      const chatMsgs: ChatMessage[] = rows
        .filter((r) => r.role === "user" || r.role === "assistant")
        .map((r, i) => ({
          id: r.id || `hist-${i}`,
          role: r.role as "user" | "assistant",
          content: r.content || "[调用工具]",
        }));
      setMessages(chatMsgs);
    }).catch(() => {
      setMessages([]);
    });
  }, [selectedTaskId]);

  const handleSend = useCallback(
    async (content: string) => {
      if (!selectedTaskId) return;
      const userMsg: ChatMessage = { id: `u-${Date.now()}`, role: "user", content };
      const assistantMsgId = `a-${Date.now()}`;
      streamingMsgIdRef.current = assistantMsgId;

      setMessages((prev) => [
        ...prev,
        userMsg,
        { id: assistantMsgId, role: "assistant", content: "" },
      ]);

      try {
        const response = await chat(content);
        const list = sessionMessagesToChatMessages(response.session.messages);
        setMessages(list);
        fetchTasks();
      } catch (e) {
        const errMsg = e instanceof Error ? e.message : "请求失败，请重试。";
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.id === assistantMsgId) {
            return [...prev.slice(0, -1), { ...last, content: errMsg }];
          }
          return [...prev, { id: `err-${Date.now()}`, role: "assistant", content: errMsg }];
        });
      } finally {
        streamingMsgIdRef.current = null;
      }
    },
    [chat, selectedTaskId, fetchTasks]
  );

  if (!user) return null;

  const noTask = !selectedTaskId;

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
        <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 8 }}>
          <button
            type="button"
            className="btn btn-default btn-small"
            onClick={() => setShowTaskDetail((v) => !v)}
          >
            {showTaskDetail ? "隐藏任务详情" : "展开任务详情"}
          </button>
        </div>

        {showTaskDetail && <TaskDetail />}

        <div className="admiral-surface" style={{ marginTop: showTaskDetail ? 12 : 0, padding: 12 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, marginBottom: 8 }}>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>Agent 对话</h3>
              <p style={{ fontSize: 12, color: "var(--admiral-text-secondary)", margin: "3px 0 0" }}>
                {noTask ? "请先在左侧选择或创建任务" : "与 AI 对话管理当前任务"}
              </p>
            </div>
            {selectedTaskId && <span className="admiral-chip admiral-chip-primary">实时会话</span>}
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
            {noTask ? (
              <p style={{ margin: "28px 0", fontSize: 13, color: "var(--admiral-text-placeholder)", textAlign: "center" }}>
                请先选择或创建任务，对话将与任务绑定。
              </p>
            ) : messages.length > 0 ? (
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
          placeholder={noTask ? "请先选择任务..." : "输入指令或描述任务..."}
          disabled={loading || noTask}
          loading={loading}
          quickActions={noTask ? [] : QUICK_ACTIONS}
        />
      </div>
    </div>
  );
}
