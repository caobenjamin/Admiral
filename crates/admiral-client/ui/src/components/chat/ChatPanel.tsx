import { useRef, useEffect } from "react";
import { MessageBubble, type ChatMessage } from "./MessageBubble";
import { ChatInput } from "./ChatInput";

interface ChatPanelProps {
  messages?: ChatMessage[];
  onSend?: (content: string) => void;
  disabled?: boolean;
  loading?: boolean;
  placeholder?: string;
}

export function ChatPanel({
  messages = [],
  onSend,
  disabled = false,
  loading = false,
  placeholder = "输入指令或描述任务…",
}: ChatPanelProps) {
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  return (
    <div
      className="flex flex-col h-full min-h-0 rounded-lg border flex-1"
      style={{
        borderColor: "var(--admiral-border)",
        backgroundColor: "var(--admiral-card)",
        borderRadius: "var(--admiral-radius-card)",
      }}
    >
      <div
        ref={listRef}
        className="flex-1 overflow-y-auto p-4 space-y-2 min-h-[120px]"
      >
        {messages.length === 0 ? (
          <p className="text-sm text-[var(--admiral-text-placeholder)] text-center py-8">
            在下方输入自然语言描述任务，或与 Agent 对话
          </p>
        ) : (
          messages.map((m) => <MessageBubble key={m.id} message={m} />)
        )}
      </div>
      <div className="p-3 border-t shrink-0" style={{ borderColor: "var(--admiral-border)" }}>
        <ChatInput
          onSend={(text) => onSend?.(text)}
          placeholder={placeholder}
          disabled={disabled}
          loading={loading}
        />
      </div>
    </div>
  );
}
