// PRD 7.2: 对话气泡（user / assistant / system / tool）

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

export function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";
  const isTool = message.role === "tool";

  if (isSystem) {
    return (
      <div className="flex justify-center my-2">
        <span
          className="text-xs px-2 py-1 rounded"
          style={{ color: "var(--admiral-text-placeholder)", backgroundColor: "var(--admiral-bg)" }}
        >
          {message.content}
        </span>
      </div>
    );
  }

  if (isTool) {
    return (
      <div className="flex justify-start my-1">
        <div
          className="max-w-[85%] rounded-lg px-3 py-2 text-sm"
          style={{
            backgroundColor: "var(--admiral-bg)",
            borderColor: "var(--admiral-border)",
            borderWidth: 1,
          }}
        >
          <span className="text-xs font-medium" style={{ color: "var(--admiral-text-secondary)" }}>
            工具
          </span>
          <pre
            className="mt-1 whitespace-pre-wrap break-words font-sans"
            style={{ color: "var(--admiral-text-primary)" }}
          >
            {message.content}
          </pre>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex my-2 ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[85%] rounded-2xl px-4 py-2.5 text-sm ${
          isUser ? "bg-primary text-white rounded-br-md" : "rounded-bl-md"
        }`}
        style={
          isUser
            ? undefined
            : {
                backgroundColor: "var(--admiral-card)",
                borderColor: "var(--admiral-border)",
                borderWidth: 1,
                color: "var(--admiral-text-primary)",
              }
        }
      >
        <span className="font-medium text-xs opacity-80 block mb-0.5">
          {isUser ? "我" : "Admiral"}
        </span>
        <div className="whitespace-pre-wrap break-words">{message.content}</div>
      </div>
    </div>
  );
}
