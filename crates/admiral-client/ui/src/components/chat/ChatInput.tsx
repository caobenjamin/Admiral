import { useState, useRef, useEffect } from "react";

interface ChatInputProps {
  onSend: (content: string) => void;
  placeholder?: string;
  disabled?: boolean;
  loading?: boolean;
}

const defaultPlaceholder = "输入指令或描述任务…";

export function ChatInput({
  onSend,
  placeholder = defaultPlaceholder,
  disabled = false,
  loading = false,
}: ChatInputProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 120)}px`;
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
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  return (
    <div
      className="flex gap-2 items-end p-3 rounded-lg border transition-colors focus-within:ring-2 focus-within:ring-primary/30"
      style={{
        borderColor: "var(--admiral-border)",
        borderRadius: "var(--admiral-radius-card)",
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
        className="flex-1 min-h-[40px] max-h-[120px] resize-none py-2 px-3 text-sm outline-none disabled:opacity-50 disabled:cursor-not-allowed bg-transparent"
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
        className="shrink-0 px-4 py-2 text-sm font-medium text-white rounded-md bg-primary hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
        style={{ borderRadius: "var(--admiral-radius-button)" }}
      >
        {loading ? "发送中…" : "发送"}
      </button>
    </div>
  );
}
