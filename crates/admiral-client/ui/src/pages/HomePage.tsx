import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAgent } from "@/hooks/useAgent";
import { useUiStore } from "@/stores/uiStore";
import { useTaskStore, type TaskMeta } from "@/stores/taskStore";
import { useToastStore } from "@/stores/toastStore";
import { TaskDetail } from "@/components/task/TaskDetail";
import { ChatInput } from "@/components/chat/ChatInput";
import { MessageBubble, type ChatMessage, type MessageRole } from "@/components/chat/MessageBubble";
import * as tauri from "@/lib/tauri";
import type { LicenseResponse } from "@/lib/types";

type LicenseStatus = "loading" | "active" | "inactive" | "check_failed";

function formatUtcDateTime(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;

  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  const seconds = String(date.getUTCSeconds()).padStart(2, "0");

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds} UTC`;
}

function getInactiveLicenseMessage(license: LicenseResponse | null): string {
  const expiresAt = formatUtcDateTime(license?.expires_at ?? null);
  if (expiresAt) {
    if (license?.type === "trial") {
      return `当前账号试用已于 ${expiresAt} 过期，暂时无法使用 AI 对话。`;
    }
    return `当前账号许可已于 ${expiresAt} 过期，暂时无法使用 AI 对话。`;
  }
  return "当前账号许可已过期，暂时无法使用 AI 对话。";
}

function mapAgentErrorMessage(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  if (raw.includes("LICENSE_INACTIVE")) {
    return "当前账号许可已过期，暂时无法使用 AI 对话。";
  }
  return raw || "请求失败，请重试。";
}

function sessionMessagesToChatMessages(
  sessionMessages: { role: string; content?: string | null; tool_calls?: unknown; name?: string | null }[]
): ChatMessage[] {
  return sessionMessages
    .filter((m) => m.role === "user" || m.role === "assistant" || m.role === "tool")
    .map((m, i) => {
      let content = m.content ?? "";
      if (m.role === "assistant" && m.tool_calls && Array.isArray(m.tool_calls) && m.tool_calls.length > 0) {
        if (!content) content = "[调用工具]";
      }
      if (m.role === "assistant" && !content.trim()) {
        content = "本轮未收到可展示的正文回复，请查看过程日志。";
      }
      if (m.role === "tool" && m.name) {
        content = `工具 ${m.name} 返回：\n${content || "{}"}`;
      }
      return {
        id: `msg-${i}`,
        role: m.role as MessageRole,
        content,
      };
    });
}

function getQuickActions(task: TaskMeta | null): string[] {
  if (!task) return [];
  switch (task.status) {
    case "draft":
    case "exploring":
      return ["描述这个任务的目标", "我想查询广告报表", "需要批量调价"];
    case "configuring":
      return ["开始试运行", "查看当前任务配置", "修改执行计划"];
    case "testing":
    case "ready":
      return ["查看试运行报告", "发布任务", "调整配置"];
    case "active":
    case "completed":
      return ["查看最近执行报告", "调整任务配置", "暂停任务"];
    case "failed":
      return ["查看失败原因", "重新配置", "重试"];
    default:
      return ["查看任务状态"];
  }
}

export function HomePage() {
  const user = useAuthStore((s) => s.user);
  const license = useAuthStore((s) => s.license);
  const setUser = useAuthStore((s) => s.setUser);
  const setLicense = useAuthStore((s) => s.setLicense);
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const { fetchTasks } = useTaskStore();
  const showToast = useToastStore((s) => s.show);
  const navigate = useNavigate();

  const tasks = useTaskStore((s) => s.tasks);
  const currentTask = useMemo(() => (selectedTaskId ? tasks.find((t) => t.id === selectedTaskId) ?? null : null), [selectedTaskId, tasks]);
  const quickActions = useMemo(() => getQuickActions(currentTask), [currentTask]);

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [showTaskDetail, setShowTaskDetail] = useState(true);
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus>("loading");
  const prevTaskIdRef = useRef<string | null>(null);
  const streamingMsgIdRef = useRef<string | null>(null);
  const processMsgIdsRef = useRef<string[]>([]);

  const appendProcessMessage = useCallback((role: MessageRole, content: string) => {
    const id = `proc-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    processMsgIdsRef.current.push(id);
    setMessages((prev) => [...prev, { id, role, content }]);
  }, []);

  const licenseNotice = useMemo(() => {
    if (licenseStatus === "loading") return "正在检查许可状态，请稍候。";
    if (licenseStatus === "inactive") return getInactiveLicenseMessage(license);
    if (licenseStatus === "check_failed") return "许可状态校验失败，请稍后重试。";
    return null;
  }, [license, licenseStatus]);

  const { chat, loading, error } = useAgent(selectedTaskId, {
    onStatus(stage, message, level) {
      const prefix = level === "error" ? "错误" : level === "warn" ? "注意" : "进度";
      appendProcessMessage("system", `${prefix} · ${stage}\n${message}`);
    },
    onDelta(content) {
      const msgId = streamingMsgIdRef.current;
      if (!msgId) return;
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.id === msgId) {
          return [...prev.slice(0, -1), { ...last, content: last.content + content }];
        }
        return prev;
      });
    },
    onToolUse(name) {
      appendProcessMessage("system", `工具调用\n${name}`);
    },
    onToolResult(name, success) {
      appendProcessMessage("system", `${success ? "工具完成" : "工具失败"}\n${name}`);
    },
    onError(message) {
      appendProcessMessage("system", `流错误\n${message}`);
      showToast(message, "error");
    },
  });

  const refreshLicenseStatus = useCallback(async (): Promise<{ status: LicenseStatus; license: LicenseResponse | null }> => {
    try {
      const license = await tauri.authLicense();
      setLicense(license);
      const nextStatus: LicenseStatus = license.is_active ? "active" : "inactive";
      setLicenseStatus(nextStatus);
      return { status: nextStatus, license };
    } catch {
      setLicense(null);
      setLicenseStatus("check_failed");
      return { status: "check_failed", license: null };
    }
  }, [setLicense]);

  useEffect(() => {
    let cancelled = false;

    const loadPageState = async () => {
      try {
        const me = await tauri.authMe();
        if (cancelled) return;
        setUser(me);
      } catch {
        if (cancelled) return;
        setUser(null);
        setLicense(null);
        navigate("/login");
        return;
      }

      if (cancelled) return;
      await refreshLicenseStatus();
    };

    void loadPageState();

    return () => {
      cancelled = true;
    };
  }, [navigate, refreshLicenseStatus, setLicense, setUser]);

  useEffect(() => {
    if (selectedTaskId === prevTaskIdRef.current) return;
    prevTaskIdRef.current = selectedTaskId;

    if (!selectedTaskId) {
      setMessages([]);
      return;
    }

    tauri.taskLoadConversations(selectedTaskId).then((rows) => {
      const chatMsgs: ChatMessage[] = rows
        .filter((r) => r.role === "user" || r.role === "assistant" || r.role === "tool")
        .map((r, i) => ({
          id: r.id || `hist-${i}`,
          role: r.role as MessageRole,
          content: r.role === "tool" && r.name ? `工具 ${r.name} 返回：\n${r.content || "{}"}` : (r.content || "[调用工具]"),
        }));
      setMessages(chatMsgs);
    }).catch(() => {
      setMessages([]);
    });
  }, [selectedTaskId]);

  const handleSend = useCallback(
    async (content: string) => {
      if (!selectedTaskId) return;

      const { status: nextLicenseStatus, license: nextLicense } = await refreshLicenseStatus();
      if (nextLicenseStatus === "loading") return;
      if (nextLicenseStatus === "inactive") {
        showToast(getInactiveLicenseMessage(nextLicense), "error");
        return;
      }
      if (nextLicenseStatus === "check_failed") {
        showToast("许可状态校验失败，请稍后重试。", "error");
        return;
      }

      const userMsg: ChatMessage = { id: `u-${Date.now()}`, role: "user", content };
      const assistantMsgId = `a-${Date.now()}`;
      streamingMsgIdRef.current = assistantMsgId;
      processMsgIdsRef.current = [];

      setMessages((prev) => [
        ...prev,
        userMsg,
        { id: assistantMsgId, role: "assistant", content: "" },
      ]);
      appendProcessMessage("system", "请求已发送\n正在建立流式会话并等待模型响应。");

      try {
        const response = await chat(content);
        const list = sessionMessagesToChatMessages(response.session.messages);
        const processIds = new Set(processMsgIdsRef.current);
        setMessages((prev) => {
          const processMessages = prev.filter((m) => processIds.has(m.id));
          return [...processMessages, ...list];
        });
        if (!response.content.trim() && response.tool_calls_used.length === 0) {
          appendProcessMessage("system", "未收到正文\n模型请求已完成，但没有返回可展示文本，也没有触发工具。");
        }
        fetchTasks();
      } catch (e) {
        const errMsg = mapAgentErrorMessage(e);
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.id === assistantMsgId) {
            return [...prev.slice(0, -1), { ...last, content: errMsg }];
          }
          return [...prev, { id: `err-${Date.now()}`, role: "assistant", content: errMsg }];
        });
        appendProcessMessage("system", `请求失败\n${errMsg}`);
      } finally {
        streamingMsgIdRef.current = null;
      }
    },
    [appendProcessMessage, chat, selectedTaskId, fetchTasks, refreshLicenseStatus, showToast]
  );

  if (!user) return null;

  const noTask = !selectedTaskId;
  const inputDisabled = loading || noTask || licenseStatus !== "active";
  const inputPlaceholder =
    licenseStatus === "loading"
      ? "正在检查许可状态..."
      : licenseStatus === "inactive"
        ? "当前账号许可已过期"
        : licenseStatus === "check_failed"
          ? "许可状态校验失败，请稍后重试"
          : noTask
            ? "请先选择任务..."
            : "输入指令或描述任务...";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        minHeight: 0,
        overflow: "hidden",
        padding: 14,
        gap: 0,
      }}
    >
      <div style={{ flex: 1, minHeight: 0, overflow: "auto", paddingBottom: 12 }}>
        <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 8 }}>
          <button
            type="button"
            className="btn btn-default btn-small"
            onClick={() => setShowTaskDetail((v) => !v)}
          >
            {showTaskDetail ? "隐藏任务详情" : "展开任务详情"}
          </button>
        </div>

        {showTaskDetail && <TaskDetail />}

        <div className="admiral-surface" style={{ marginTop: showTaskDetail ? 12 : 0, padding: 12 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 12, marginBottom: 8 }}>
            <div>
              <h3 style={{ fontSize: 14, fontWeight: 700, margin: 0, color: "var(--admiral-text-primary)" }}>Agent 对话</h3>
              <p style={{ fontSize: 12, color: "var(--admiral-text-secondary)", margin: "3px 0 0" }}>
                {noTask ? "请先在左侧选择或创建任务" : "与 AI 对话定义和管理任务"}
              </p>
            </div>
            {currentTask && currentTask.completeness !== "complete" && (
              <span style={{ fontSize: 12, color: "var(--admiral-text-placeholder)" }}>
                {currentTask.completeness === "incomplete" && "请描述任务目标..."}
                {currentTask.completeness === "partial" && "继续补充信息..."}
                {currentTask.completeness === "sufficient" && "信息基本完整，可试运行"}
              </span>
            )}
            {selectedTaskId && licenseStatus === "active" && <span className="admiral-chip admiral-chip-primary">实时会话</span>}
            {licenseStatus === "loading" && <span className="admiral-chip">检查许可中</span>}
            {licenseStatus === "inactive" && <span className="admiral-chip admiral-chip-danger">许可已失效</span>}
            {licenseStatus === "check_failed" && <span className="admiral-chip admiral-chip-danger">许可校验失败</span>}
          </div>

          {licenseNotice && (
            <div
              style={{
                marginBottom: 8,
                padding: "10px 12px",
                borderRadius: 10,
                border: "1px solid var(--admiral-border)",
                backgroundColor: licenseStatus === "check_failed" ? "#fff7e8" : "#fff3f0",
                color: "var(--admiral-text-primary)",
                fontSize: 13,
              }}
            >
              {licenseNotice}
            </div>
          )}

          <div
            style={{
              border: "1px solid var(--admiral-border)",
              borderRadius: 12,
              backgroundColor: "#fbfdff",
              minHeight: 130,
              maxHeight: 430,
              overflowY: "auto",
              padding: "10px 12px",
            }}
          >
            {noTask ? (
              <p style={{ margin: "28px 0", fontSize: 13, color: "var(--admiral-text-placeholder)", textAlign: "center" }}>
                请先选择或创建任务，对话将与任务绑定。
              </p>
            ) : messages.length > 0 ? (
              messages.map((m) => <MessageBubble key={m.id} message={m} />)
            ) : (
              <div style={{ margin: "28px 16px", textAlign: "center" }}>
                <p style={{ fontSize: 14, color: "var(--admiral-text-secondary)", margin: "0 0 8px" }}>
                  告诉我这个任务想要做什么吧
                </p>
                <p style={{ fontSize: 12, color: "var(--admiral-text-placeholder)", margin: 0 }}>
                  你可以描述任务目标，系统会引导你逐步完善配置
                </p>
              </div>
            )}
          </div>

          {error && (
            <div className="admiral-chip admiral-chip-danger" style={{ marginTop: 8 }}>
              {error}
            </div>
          )}
        </div>
      </div>

      <div
        style={{
          flexShrink: 0,
          borderTop: "1px solid var(--admiral-border)",
          paddingTop: 12,
          backgroundColor: "transparent",
        }}
      >
        {licenseNotice && (
          <div
            style={{
              marginBottom: 8,
              padding: "10px 12px",
              borderRadius: 10,
              border: "1px solid var(--admiral-border)",
              backgroundColor: licenseStatus === "check_failed" ? "#fff7e8" : "#fff3f0",
              color: "var(--admiral-text-primary)",
              fontSize: 13,
            }}
          >
            {licenseNotice}
          </div>
        )}
        <ChatInput
          onSend={handleSend}
          placeholder={inputPlaceholder}
          disabled={inputDisabled}
          loading={loading}
          quickActions={quickActions}
        />
      </div>
    </div>
  );
}
