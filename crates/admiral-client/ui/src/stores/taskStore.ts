import { create } from "zustand";

// 任务列表状态（PRD 12.3）。MVP 阶段可用 mock，后续接真实 API。

export type TaskStatus =
  | "draft"
  | "dry_running"
  | "dry_run_passed"
  | "test_run_ready"
  | "test_run_running"
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
  type: "instant" | "scheduled";
  updated_at: string;
  last_run_result?: "success" | "failed" | null;
}

interface TaskState {
  tasks: TaskMeta[];
  setTasks: (tasks: TaskMeta[]) => void;
  fetchTasks: () => Promise<void>;
  createTask: (params: { name: string; type: "instant" | "scheduled" }) => Promise<TaskMeta>;
}

// 占位：暂无后端接口时返回 mock 数据，后续接真实 API
async function fetchTasksFromApi(): Promise<TaskMeta[]> {
  return [
    { id: "1", name: "示例任务 A", status: "running", type: "instant", updated_at: new Date().toISOString(), last_run_result: "success" },
    { id: "2", name: "示例任务 B", status: "succeeded", type: "instant", updated_at: new Date().toISOString(), last_run_result: "success" },
    { id: "3", name: "示例任务 C", status: "failed", type: "scheduled", updated_at: new Date().toISOString(), last_run_result: "failed" },
  ];
}

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  setTasks: (tasks) => set({ tasks }),
  fetchTasks: async () => {
    const list = await fetchTasksFromApi();
    set({ tasks: list });
  },
  createTask: async (params) => {
    const newTask: TaskMeta = {
      id: `task-${Date.now()}`,
      name: params.name,
      status: "draft",
      type: params.type,
      updated_at: new Date().toISOString(),
    };
    set({ tasks: [...get().tasks, newTask] });
    return newTask;
  },
}));
