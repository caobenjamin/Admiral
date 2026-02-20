import { create } from "zustand";

// UI 状态：连接状态、选中的任务、LeftTab 等（PRD 7.2）

export type ConnectionStatus = "online" | "offline" | "connecting";

export type ActiveTab = "media" | "tasks" | "skills" | "other";

interface UiState {
  connectionStatus: ConnectionStatus;
  setConnectionStatus: (s: ConnectionStatus) => void;
  selectedTaskId: string | null;
  setSelectedTaskId: (id: string | null) => void;
  activeTab: ActiveTab;
  setActiveTab: (tab: ActiveTab) => void;
  leftTabCollapsed: boolean;
  setLeftTabCollapsed: (v: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  connectionStatus: "online",
  setConnectionStatus: (connectionStatus) => set({ connectionStatus }),
  selectedTaskId: null,
  setSelectedTaskId: (selectedTaskId) => set({ selectedTaskId }),
  activeTab: "tasks",
  setActiveTab: (activeTab) => set({ activeTab }),
  leftTabCollapsed: false,
  setLeftTabCollapsed: (leftTabCollapsed) => set({ leftTabCollapsed }),
}));
