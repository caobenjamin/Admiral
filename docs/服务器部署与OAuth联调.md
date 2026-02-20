# 服务器部署与 OAuth 联调

本仓库采用「**服务端在远端运行、本地仅做代码编辑**」的联调模式。服务端需部署到具备 HTTPS 的远端服务器，客户端在本地运行并连接远端 API。

B 站 OAuth 要求**授权回调地址为 HTTPS**，本地 `localhost` 无法满足，因此服务端必须部署到具备 HTTPS 的服务器上。

## 一、问题说明

- 发起授权后，浏览器会跳转到 B 站的授权页，授权成功后再跳回你配置的 **Callback URL**。
- B 站要求该回调地址必须为 **HTTPS**；若使用 `https://localhost:8080/...`，浏览器会报 `ERR_SSL_PROTOCOL_ERROR`（本地没有有效的 HTTPS）。
- 解决方式：**将 Admiral 服务端部署到已有 HTTPS 的服务器**，把回调地址改为 `https://你的域名/api/v1/oauth/bilibili/callback`。

---

## 二、服务端部署要点

### 1. 服务器与环境

- 一台有公网 IP 的服务器（或内网 + 端口转发）。
- 域名解析到该服务器（例如 `api.yourdomain.com`）。
- 使用 **Nginx / Caddy / 其他反向代理** 做 HTTPS 终结（申请证书，如 Let’s Encrypt），并反向代理到 Admiral 后端（例如 `127.0.0.1:8080`）。

### 2. 服务端环境变量（.env）

在服务器上部署时，重点修改/确认以下项：

| 变量 | 说明 | 示例（服务器） |
|------|------|----------------|
| `ADMIRAL_DATABASE_URL` | PostgreSQL 连接串（服务器本机或云数据库） | `postgres://user:pass@localhost:5432/admiral` 或 RDS 等 |
| `ADMIRAL_JWT_PRIVATE_KEY_PATH` | JWT 私钥路径 | `./keys/private.pem` 或绝对路径 |
| `ADMIRAL_JWT_PUBLIC_KEY_PATH` | JWT 公钥路径 | `./keys/public.pem` |
| `ADMIRAL_TOKEN_ENCRYPTION_KEY` | Token 加密密钥（32 字节 hex） | 与本地可一致或重新生成 |
| **`ADMIRAL_BILI_CALLBACK_URL`** | **B 站 OAuth 回调地址（必须 HTTPS）** | **`https://api.yourdomain.com/api/v1/oauth/bilibili/callback`** |
| `ADMIRAL_BILI_APP_ID` / `ADMIRAL_BILI_APP_SECRET` | B 站应用密钥 | 与 B 站开放平台一致 |
| `ADMIRAL_SERVER_HOST` | 监听地址 | 若前面有反向代理，填 `127.0.0.1` 即可 |
| `ADMIRAL_SERVER_PORT` | 监听端口 | 如 `8080`（给 Nginx 反向代理用） |

**重要**：`ADMIRAL_BILI_CALLBACK_URL` 必须与你在 **B 站开放平台** 里为该应用配置的「授权回调页」**完全一致**（含路径、无多余斜杠）。

### 3. B 站开放平台配置

1. 登录 [B 站开放平台](https://open.bilibili.com/)（或你使用的 B 站广告/创作者应用后台）。
2. 找到你的应用（APP_ID 对应 `.env` 里的 `ADMIRAL_BILI_APP_ID`）。
3. 在「授权设置」或「OAuth 回调地址」中，添加一条：
   - **授权回调页**：`https://你的域名/api/v1/oauth/bilibili/callback`
4. 保存后，再在 Admiral 客户端里发起授权，回调即可走 HTTPS。

### 4. 反向代理示例（Nginx）

Admiral 服务端只监听 HTTP（如 `127.0.0.1:8080`），由 Nginx 对外提供 HTTPS：

```nginx
server {
    listen 443 ssl;
    server_name api.yourdomain.com;

    ssl_certificate     /path/to/fullchain.pem;
    ssl_certificate_key  /path/to/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

部署后确认：`curl -I https://api.yourdomain.com/health` 返回 200。

---

## 三、客户端联调（指向远端服务器）

本地运行 **Tauri 客户端** 进行联调时，需让客户端请求**远端服务器上的 API**：

- 设置环境变量 **`ADMIRAL_API_BASE_URL`** = 服务器 API 的 HTTPS 地址（无路径、无末尾斜杠），例如：
  - `export ADMIRAL_API_BASE_URL=https://api.yourdomain.com`
- 若 Nginx 未改端口，则一般为 **443**，URL 中不必写 `:443`。

然后在本地执行：

```bash
cd crates/admiral-client
export ADMIRAL_API_BASE_URL=https://api.yourdomain.com
cargo tauri dev
```

客户端会向该地址发起登录、OAuth 发起/回调、MAPI、LLM 等请求；**授权后浏览器会跳转到服务器的 HTTPS 回调地址**，不再出现 `ERR_SSL_PROTOCOL_ERROR`。

---

## 四、自检清单（部署后）

1. [ ] 服务器上已部署 Admiral 服务端，并配置好 `.env`（含 `ADMIRAL_BILI_CALLBACK_URL=https://你的域名/api/v1/oauth/bilibili/callback`）。
2. [ ] B 站开放平台中该应用的「授权回调页」已设为上述 URL 且已保存。
3. [ ] `https://你的域名/health` 可访问且返回 200。
4. [ ] 本地客户端已设置 `ADMIRAL_API_BASE_URL=https://你的域名` 并重启。
5. [ ] 在客户端中发起 B 站授权，能在浏览器中完成授权并正常跳回，不再出现 `ERR_SSL_PROTOCOL_ERROR`。

完成以上后，即可在「远端服务器 + 本地客户端」的组合下继续完成 OAuth、MAPI、LLM 等联调。
