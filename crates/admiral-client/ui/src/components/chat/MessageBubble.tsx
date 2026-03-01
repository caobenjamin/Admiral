import { useMemo, useState } from "react";

export type MessageRole = "user" | "assistant" | "system" | "tool";

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp?: string;
}

interface MessageBubbleProps {
  message: ChatMessage;
}

function summarizeText(text: string): string {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized) return "无内容";
  if (normalized.length <= 130) return normalized;
  return `${normalized.slice(0, 130)}...`;
}

function prettifyToolContent(text: string): string {
  try {
    const obj = JSON.parse(text);
    return JSON.stringify(obj, null, 2);
  } catch {
    return text;
  }
}

export function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";
  const isTool = message.role === "tool";
  const isAssistant = message.role === "assistant";

  const [expanded, setExpanded] = useState(false);
  const [toolExpanded, setToolExpanded] = useState(false);

  const toolRaw = useMemo(() => prettifyToolContent(message.content), [message.content]);
  const assistantLong = isAssistant && message.content.length > 360;

  if (isSystem) {
    return (
      <div style={{ display: "flex", justifyContent: "center", margin: "8px 0" }}>
        <span
          style={{
            fontSize: 12,
            padding: "4px 10px",
            borderRadius: 999,
            color: "var(--admiral-text-placeholder)",
            backgroundColor: "var(--admiral-bg-strong)",
            border: "1px solid var(--admiral-border)",
          }}
        >
          {message.content}
        </span>
      </div>
    );
  }

  if (isTool) {
    return (
      <div style={{ display: "flex", justifyContent: "flex-start", margin: "6px 0" }}>
        <div
          style={{
            width: "min(100%, 92%)",
            borderRadius: 12,
            padding: "10px 12px",
            backgroundColor: "#f8fbff",
            border: "1px solid var(--admiral-border)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10, marginBottom: 6 }}>
            <span className="admiral-chip admiral-chip-primary">工具输出</span>
            <button
              type="button"
              className="btn btn-default btn-small"
              onClick={() => setToolExpanded((v) => !v)}
            >
              {toolExpanded ? "收起原文" : "查看原文"}
            </button>
          </div>

          <div style={{ fontSize: 12, color: "var(--admiral-text-secondary)", marginBottom: toolExpanded ? 8 : 0 }}>
            {summarizeText(message.content)}
          </div>

          {toolExpanded && (
            <pre
              className="message-tool-content"
              style={{
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
                color: "var(--admiral-text-primary)",
                border: "1px solid var(--admiral-border)",
                borderRadius: 10,
                background: "#ffffff",
                padding: 10,
                maxHeight: 220,
                overflow: "auto",
              }}
            >
              {toolRaw}
            </pre>
          )}
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", margin: "10px 0", justifyContent: isUser ? "flex-end" : "flex-start" }}>
      <div
        style={{
          width: "min(100%, 88%)",
          padding: "10px 12px",
          borderRadius: 14,
          ...(isUser
            ? {
                background: "linear-gradient(135deg, #0f6cbd 0%, #0b5ca4 100%)",
                color: "#fff",
                borderBottomRightRadius: 6,
                boxShadow: "0 12px 24px rgba(15, 108, 189, 0.2)",
              }
            : {
                backgroundColor: "#ffffff",
                border: "1px solid var(--admiral-border)",
                color: "var(--admiral-text-primary)",
                borderBottomLeftRadius: 6,
                boxShadow: "var(--admiral-shadow-soft)",
              }),
        }}
      >
        <span style={{ fontSize: 12, fontWeight: 700, opacity: isUser ? 0.9 : 1, display: "block", marginBottom: 3 }}>
          {isUser ? "我" : "Admiral"}
        </span>

        {assistantLong && !expanded ? (
          <div>
            <div className="message-fade" style={{ maxHeight: 130, overflow: "hidden", whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
              {message.content}
            </div>
            <button
              type="button"
              className="btn btn-default btn-small"
              style={{ marginTop: 8 }}
              onClick={() => setExpanded(true)}
            >
              展开全文
            </button>
          </div>
        ) : (
          <div style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{message.content}</div>
        )}

        {assistantLong && expanded && (
          <button
            type="button"
            className="btn btn-default btn-small"
            style={{ marginTop: 8 }}
            onClick={() => setExpanded(false)}
          >
            收起
          </button>
        )}
      </div>
    </div>
  );
}
