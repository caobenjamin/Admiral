import { create } from "zustand";
import * as tauri from "@/lib/tauri";
import type { Task } from "@/lib/types";

// 任务列表状态（PRD 12.3），对接真实 API。

export type TaskStatus =
  | "draft"
  | "planning"
  | "plan_ready"
  | "dry_running"
  | "dry_run_passed"
  | "test_running"
  | "test_run_passed"
  | "deployed"
  | "running"
  | "scheduled"
  | "paused"
  | "succeeded"
  | "failed";

export interface TaskMeta {
  id: string;
  name: string;
  status: TaskStatus;
  type: "instant" | "scheduled" | "heartbeat";
  updated_at: string;
  last_run_result?: "success" | "failed" | null;
}

function taskToMeta(t: Task): TaskMeta {
  return {
    id: t.id,
    name: t.name,
    status: t.status as TaskStatus,
    type: t.task_type,
    updated_at: t.updated_at,
    last_run_result: undefined,
  };
}

interface TaskState {
  tasks: TaskMeta[];
  setTasks: (tasks: TaskMeta[]) => void;
  fetchTasks: () => Promise<void>;
  createTask: (params: { name: string; type: "instant" | "scheduled" }) => Promise<TaskMeta>;
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  setTasks: (tasks) => set({ tasks }),
  fetchTasks: async () => {
    try {
      const list = await tauri.taskList(null);
      set({ tasks: list.map(taskToMeta) });
    } catch {
      set({ tasks: [] });
    }
  },
  createTask: async (params) => {
    const task = await tauri.taskCreate(
      params.name,
      null,
      params.type === "scheduled" ? "cron" : "instant"
    );
    const meta = taskToMeta(task);
    set({ tasks: [...get().tasks, meta] });
    return meta;
  },
}));
