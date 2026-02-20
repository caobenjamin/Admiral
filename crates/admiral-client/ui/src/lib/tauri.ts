// Tauri invoke wrappers for commands

import { invoke } from "@tauri-apps/api/core";
import type {
  TokenResponse,
  UserInfo,
  LicenseResponse,
  InitiateResponse,
  TokenMeta,
  OAuthAccountsResponse,
} from "./types";

export async function authRegister(req: {
  email: string;
  password: string;
  name: string;
  company?: string;
  invite_code: string;
}): Promise<TokenResponse> {
  return invoke("auth_register", { req });
}

export async function authLogin(req: {
  email: string;
  password: string;
}): Promise<TokenResponse> {
  return invoke("auth_login", { req });
}

export async function authRefresh(): Promise<{ access_token: string; expires_in: number; token_type: string }> {
  return invoke("auth_refresh");
}

export async function authLogout(): Promise<void> {
  return invoke("auth_logout");
}

export async function authMe(): Promise<UserInfo> {
  return invoke("auth_me");
}

export async function authLicense(): Promise<LicenseResponse> {
  return invoke("auth_license");
}

export async function oauthInitiate(): Promise<InitiateResponse> {
  return invoke("oauth_initiate");
}

export async function oauthStatus(): Promise<TokenMeta[]> {
  return invoke("oauth_status");
}

export async function oauthBindingsList(): Promise<
  import("./types").BoundAccountRecord[]
> {
  return invoke("oauth_bindings_list");
}

export async function oauthAccountsList(oauthSessionId?: string): Promise<OAuthAccountsResponse> {
  return invoke("oauth_accounts_list", { oauthSessionId });
}

export async function oauthBindAccounts(oauthTokenId: string, accountIds: number[]): Promise<void> {
  return invoke("oauth_bind_accounts", { req: { oauth_token_id: oauthTokenId, account_ids: accountIds } });
}

export async function oauthDeleteSession(sessionId: string): Promise<void> {
  return invoke("oauth_delete_session", { sessionId });
}

export async function mapiProxy(req: {
  account_id: number;
  method: string;
  path: string;
  query?: Record<string, unknown>;
  body?: Record<string, unknown>;
}): Promise<[number, number[]]> {
  return invoke("mapi_proxy", { req });
}

export async function llmChat(
  messages: { role: string; content: string }[],
  stream?: boolean,
  model?: string,
  maxTokens?: number
): Promise<number[]> {
  return invoke("llm_chat", {
    req: { messages, stream: stream ?? false, model, max_tokens: maxTokens },
  });
}

/** 当前客户端使用的服务端地址（用于排查是否连错服务器） */
export async function getServerUrl(): Promise<string> {
  return invoke("get_server_url");
}

/** 持久化配置文件路径，修改其中的 api_base_url 可更换服务端地址 */
export async function getConfigPath(): Promise<string | null> {
  return invoke("get_config_path");
}
