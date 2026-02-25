// Tasks hook: CRUD operations, status updates

import { useCallback, useState } from "react";
import * as tauri from "../lib/tauri";
import type { Task, TaskStatus, TaskRun } from "../lib/types";

export function useTasks(statusFilter?: TaskStatus | null) {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await tauri.taskList(statusFilter ?? undefined);
      setTasks(list);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [statusFilter]);

  const createTask = useCallback(
    async (name: string, description?: string | null, taskType: Task["task_type"] = "instant") => {
      const task = await tauri.taskCreate(name, description, taskType);
      setTasks((prev) => [...prev, task]);
      return task;
    },
    []
  );

  const getTask = useCallback(async (id: string) => {
    return tauri.taskGet(id);
  }, []);

  const runDryRun = useCallback(async (taskId: string): Promise<TaskRun> => {
    return tauri.taskRunDryRun(taskId);
  }, []);

  const runTestRun = useCallback(async (taskId: string): Promise<TaskRun> => {
    return tauri.taskRunTestRun(taskId);
  }, []);

  const runProduction = useCallback(async (taskId: string): Promise<TaskRun> => {
    return tauri.taskRunProduction(taskId);
  }, []);

  return {
    tasks,
    loading,
    error,
    refresh,
    createTask,
    getTask,
    runDryRun,
    runTestRun,
    runProduction,
  };
}
