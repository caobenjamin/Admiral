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
export type TaskStatus =
  | "draft"
  | "planning"
  | "plan_ready"
  | "dry_running"
  | "dry_run_passed"
  | "test_running"
  | "test_run_passed"
  | "deployed"
  | "scheduled"
  | "running"
  | "paused"
  | "succeeded"
  | "failed";

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
  status: TaskStatus;
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

export interface TaskRun {
  id: string;
  task_id: string;
  run_type: "dry_run" | "test_run" | "production";
  status: "running" | "succeeded" | "failed" | "cancelled";
  started_at: string;
  finished_at?: string | null;
  step_results: StepResult[];
  error?: string | null;
}

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
