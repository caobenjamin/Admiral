import { create } from "zustand";
import * as tauri from "@/lib/tauri";
import type { Task, TaskStatus, TaskCategory } from "@/lib/types";

export type { TaskStatus } from "@/lib/types";

export interface TaskMeta {
  id: string;
  name: string;
  status: TaskStatus;
  task_category: TaskCategory;
  type: "instant" | "scheduled" | "heartbeat";
  updated_at: string;
  last_run_result?: "success" | "failed" | null;
}

function taskToMeta(t: Task): TaskMeta {
  const type = t.task_type === "cron" ? "scheduled" : t.task_type;
  return {
    id: t.id,
    name: t.name,
    status: t.status,
    task_category: t.task_category,
    type,
    updated_at: t.updated_at,
    last_run_result: undefined,
  };
}

interface TaskState {
  tasks: TaskMeta[];
  setTasks: (tasks: TaskMeta[]) => void;
  fetchTasks: () => Promise<void>;
  createTask: (params: {
    name: string;
    type: "instant" | "scheduled";
    category?: TaskCategory;
  }) => Promise<TaskMeta>;
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
      params.type === "scheduled" ? "cron" : "instant",
      params.category ?? null
    );
    const meta = taskToMeta(task);
    set({ tasks: [...get().tasks, meta] });
    return meta;
  },
}));
