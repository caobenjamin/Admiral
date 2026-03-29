import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as tauri from "../lib/tauri";
import type { AgentResponse, StreamEvent } from "../lib/types";

export interface UseAgentOptions {
  onStatus?: (stage: string, message: string, level?: "info" | "warn" | "error") => void;
  onDelta?: (content: string) => void;
  onToolUse?: (name: string) => void;
  onToolResult?: (name: string, success: boolean) => void;
  onDone?: () => void;
  onError?: (message: string) => void;
}

export function useAgent(taskId?: string | null, options?: UseAgentOptions) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    return () => {
      unlistenRef.current?.();
    };
  }, []);

  const chat = useCallback(
    async (message: string): Promise<AgentResponse> => {
      setLoading(true);
      setError(null);

      unlistenRef.current?.();
      let hasVisibleStreamSignal = false;

      const unlisten = await listen<StreamEvent>("agent-stream", (event) => {
        const opts = optionsRef.current;
        const payload = event.payload;
        switch (payload.type) {
          case "status":
            hasVisibleStreamSignal = true;
            opts?.onStatus?.(payload.stage, payload.message, payload.level);
            break;
          case "delta":
            hasVisibleStreamSignal = true;
            opts?.onDelta?.(payload.content);
            break;
          case "tool_use":
            hasVisibleStreamSignal = true;
            opts?.onToolUse?.(payload.name);
            break;
          case "tool_result":
            hasVisibleStreamSignal = true;
            opts?.onToolResult?.(payload.name, payload.success);
            break;
          case "done":
            opts?.onDone?.();
            break;
          case "error":
            hasVisibleStreamSignal = true;
            opts?.onError?.(payload.message);
            break;
        }
      });
      unlistenRef.current = unlisten;

      const waitingHintTimer = window.setTimeout(() => {
        if (!hasVisibleStreamSignal) {
          optionsRef.current?.onStatus?.("waiting_first_chunk", "已建立流连接，正在等待模型返回首个片段…", "info");
        }
      }, 1200);

      try {
        const response = await tauri.agentChatStream(message, taskId ?? undefined);
        return response;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        throw e;
      } finally {
        window.clearTimeout(waitingHintTimer);
        unlisten();
        unlistenRef.current = null;
        setLoading(false);
      }
    },
    [taskId]
  );

  return { chat, loading, error };
}
