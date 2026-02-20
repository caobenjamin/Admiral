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
