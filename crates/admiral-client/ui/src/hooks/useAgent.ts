// Agent hook: chat interaction with backend Agent Runtime

import { useCallback, useState } from "react";
import * as tauri from "../lib/tauri";
import type { AgentResponse } from "../lib/types";

export function useAgent(taskId?: string | null) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const chat = useCallback(
    async (message: string): Promise<AgentResponse> => {
      setLoading(true);
      setError(null);
      try {
        const response = await tauri.agentChat(message, taskId ?? undefined);
        return response;
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setError(msg);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [taskId]
  );

  return { chat, loading, error };
}
