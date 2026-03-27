import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as tauri from "../lib/tauri";
import type { AgentResponse, StreamEvent } from "../lib/types";

export interface UseAgentOptions {
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

      const unlisten = await listen<StreamEvent>("agent-stream", (event) => {
        const opts = optionsRef.current;
        const payload = event.payload;
        switch (payload.type) {
          case "delta":
            opts?.onDelta?.(payload.content);
            break;
          case "tool_use":
            opts?.onToolUse?.(payload.name);
            break;
          case "tool_result":
            opts?.onToolResult?.(payload.name, payload.success);
            break;
          case "done":
            opts?.onDone?.();
            break;
          case "error":
            opts?.onError?.(payload.message);
            break;
        }
      });
      unlistenRef.current = unlisten;

      try {
        const response = await tauri.agentChatStream(message, taskId ?? undefined);
        return response;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        throw e;
      } finally {
        unlisten();
        unlistenRef.current = null;
        setLoading(false);
      }
    },
    [taskId]
  );

  return { chat, loading, error };
}
