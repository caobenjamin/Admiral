import { useState, useRef, useEffect } from "react";

interface ChatInputProps {
  onSend: (content: string) => void;
  placeholder?: string;
  disabled?: boolean;
  loading?: boolean;
  quickActions?: string[];
}

const defaultPlaceholder = "输入指令或描述任务...";

export function ChatInput({
  onSend,
  placeholder = defaultPlaceholder,
  disabled = false,
  loading = false,
  quickActions = [],
}: ChatInputProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 130)}px`;
  }, [value]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleSubmit = () => {
    const text = value.trim();
    if (!text || disabled || loading) return;
    onSend(text);
    setValue("");
    if (textareaRef.current) textareaRef.current.style.height = "auto";
  };

  const applyQuickAction = (template: string) => {
    setValue((prev) => (prev ? `${prev}\n${template}` : template));
    textareaRef.current?.focus();
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {quickActions.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          {quickActions.map((item) => (
            <button
              key={item}
              type="button"
              className="admiral-chip"
              style={{ cursor: "pointer" }}
              onClick={() => applyQuickAction(item)}
              disabled={disabled || loading}
            >
              {item}
            </button>
          ))}
        </div>
      )}

      <div
        className="chat-input-focus admiral-field"
        style={{
          display: "flex",
          gap: 8,
          alignItems: "flex-end",
          padding: 8,
        }}
      >
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={disabled}
          rows={1}
          className="flex-1 min-h-[38px] max-h-[130px] resize-none py-2 px-3 text-sm outline-none disabled:opacity-50 disabled:cursor-not-allowed bg-transparent"
          style={{
            borderRadius: "var(--admiral-radius-input)",
            color: "var(--admiral-text-primary)",
          }}
          aria-label="输入消息"
        />
        <button
          type="button"
          onClick={handleSubmit}
          disabled={disabled || loading || !value.trim()}
          className="btn btn-primary shrink-0"
          style={{ minWidth: 74 }}
        >
          {loading ? "发送中" : "发送"}
        </button>
      </div>
    </div>
  );
}
