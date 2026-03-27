// Tasks hook: CRUD operations, status updates, approval, resume, permissions

import { useCallback, useState } from "react";
import * as tauri from "../lib/tauri";
import type {
  Task,
  TaskStatus,
  TaskRun,
  TaskCategory,
  PendingApproval,
  OperationPermission,
} from "../lib/types";

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
    async (
      name: string,
      description?: string | null,
      taskType: Task["task_type"] = "instant",
      taskCategory?: TaskCategory | null
    ) => {
      const task = await tauri.taskCreate(name, description, taskType, taskCategory);
      setTasks((prev) => [...prev, task]);
      return task;
    },
    []
  );

  const deleteTask = useCallback(
    async (id: string) => {
      await tauri.taskDelete(id);
      setTasks((prev) => prev.filter((t) => t.id !== id));
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

  const resumeTask = useCallback(
    async (taskId: string, fromStep?: number | null, runType?: string | null): Promise<TaskRun> => {
      return tauri.taskResume(taskId, fromStep, runType);
    },
    []
  );

  const approveOperation = useCallback(
    async (taskId: string, autoApproveFuture: boolean): Promise<TaskRun> => {
      return tauri.taskApproveOperation(taskId, autoApproveFuture);
    },
    []
  );

  const rejectOperation = useCallback(async (taskId: string): Promise<Task> => {
    return tauri.taskRejectOperation(taskId);
  }, []);

  const getPendingApproval = useCallback(
    async (taskId: string): Promise<PendingApproval | null> => {
      return tauri.taskGetPendingApproval(taskId);
    },
    []
  );

  const listPermissions = useCallback(async (): Promise<OperationPermission[]> => {
    return tauri.permissionList();
  }, []);

  const revokePermission = useCallback(async (id: string): Promise<void> => {
    return tauri.permissionRevoke(id);
  }, []);

  const transitionTask = useCallback(
    async (taskId: string, newStatus: import("../lib/types").TaskStatus) => {
      return tauri.taskTransition(taskId, newStatus);
    },
    []
  );

  return {
    tasks,
    loading,
    error,
    refresh,
    createTask,
    deleteTask,
    getTask,
    runDryRun,
    runTestRun,
    runProduction,
    resumeTask,
    approveOperation,
    rejectOperation,
    getPendingApproval,
    listPermissions,
    revokePermission,
    transitionTask,
  };
}
