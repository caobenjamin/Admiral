// Shared types (mirror admiral-types API DTOs)

export interface UserInfo {
  id: string;
  email: string;
  name: string;
  role: string;
}

export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  user: UserInfo;
}

export interface RefreshResponse {
  access_token: string;
  expires_in: number;
  token_type: string;
}

export interface LicenseResponse {
  is_active: boolean;
  expires_at: string | null;
  type: string | null;
}

export interface TokenMeta {
  id: string;
  platform: string;
  created_at: string;
  bid?: number | null;
  user_name?: string | null;
}

export interface InitiateResponse {
  authorization_url: string;
}

/** B 站 OAuth 返回的 subject（组织），拉取广告主列表前需先选一个 */
export interface BiliSubjectRecord {
  subject_type: number;
  subject_id: number;
}

export interface BiliAdvertiserRecord {
  account_id: number;
  account_name: string | null;
  account_role: string | null;
}

export interface OAuthAccountsResponse {
  records: BiliAdvertiserRecord[];
  total: number;
}

/** 已绑定的广告账户 */
export interface BoundAccountRecord {
  account_id: number;
  account_name: string | null;
  platform: string;
  created_at: string;
}

// ---------- Task ----------
export type TaskType = "instant" | "cron" | "heartbeat";

export type TaskCategory = "data_fetch" | "create" | "update" | "mixed";

export type TaskStatus =
  | "draft"
  | "exploring"
  | "configuring"
  | "testing"
  | "ready"
  | "active"
  | "running"
  | "waiting_for_approval"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export type CompletenessLevel = "incomplete" | "partial" | "sufficient" | "complete";

export interface TaskStep {
  seq: number;
  skill_id: string;
  params: Record<string, unknown>;
  depends_on?: number[];
  description?: string | null;
}

export interface TaskPlan {
  steps: TaskStep[];
  estimated_duration_secs?: number;
  risk_level?: string;
}

export interface Task {
  id: string;
  name: string;
  description: string | null;
  task_type: TaskType;
  task_category: TaskCategory;
  status: TaskStatus;
  completeness: CompletenessLevel;
  plan: TaskPlan | null;
  platform: string;
  account_ids: string[];
  trigger: {
    trigger_type: string;
    cron_expression?: string | null;
    heartbeat_condition?: string | null;
    next_run_at?: string | null;
  };
  created_at: string;
  updated_at: string;
}

export interface StepResult {
  step_seq: number;
  skill_id: string;
  success: boolean;
  result?: unknown;
  error?: string | null;
  duration_ms?: number | null;
}

export type RunType = "dry_run" | "test_run" | "production";

export type RunStatus =
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "waiting_for_approval"
  | "partially_succeeded";

export interface TaskRun {
  id: string;
  task_id: string;
  run_type: RunType;
  status: RunStatus;
  started_at: string;
  finished_at?: string | null;
  step_results: StepResult[];
  error?: string | null;
  resume_from_step?: number | null;
  completed_steps: number[];
}

export interface PendingApproval {
  task_id: string;
  run_id: string;
  step_seq: number;
  skill_id: string;
  operation: string;
  params_summary: string;
}

export type PermissionScope = "platform" | "task";

export interface OperationPermission {
  id: string;
  scope_type: PermissionScope;
  scope_id: string;
  operation: string;
  auto_approve: boolean;
  granted_at: string;
  expires_at?: string | null;
}

// ---------- Stream Events ----------
export type StreamEvent =
  | { type: "delta"; content: string }
  | { type: "status"; stage: string; message: string; level?: "info" | "warn" | "error" }
  | { type: "tool_use"; name: string }
  | { type: "tool_result"; name: string; success: boolean }
  | { type: "done" }
  | { type: "error"; message: string };

// ---------- Agent ----------
export interface AgentResponse {
  content: string;
  tool_calls_used: [string, { success: boolean; data?: unknown; error?: string | null }][];
  session: {
    id: string;
    task_id: string | null;
    messages: { role: string; content?: string | null; tool_calls?: unknown; name?: string | null; tool_call_id?: string | null }[];
  };
}
