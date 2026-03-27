# Admiral Server 部署到 GCP（从零到运行）

**前提**：代码已推送到 GitHub，能 SSH 登录 GCP 服务器，域名 `admiral.example.com` 已解析到 `203.0.113.10`。

**目标**：在远端服务器上配置环境、编译、检查运行状况，在云端启动服务端，并通过 **80/443 + HTTPS** 对外提供访问。

**联调模式**：本仓库采用「服务端在远端运行、本地仅做代码编辑」的方式。服务端部署并运行在 GCP 等远端服务器，客户端在本地运行，通过 `ADMIRAL_API_BASE_URL` 连接远端 API。

---

## 关于端口与 HTTPS、Docker

- **80/443 已放行时，不需要再开放 8080。** 推荐做法：应用只监听本机 **127.0.0.1:8080**，用 **Nginx** 在 80/443 做反向代理并终结 HTTPS，这样对外只暴露 80/443，更安全，也便于用 Let's Encrypt 申请证书。
- **HTTPS**：下文会通过 Nginx + **Certbot** 为 `admiral.example.com` 申请免费证书，实现 `https://admiral.example.com` 访问。
- **Docker**：**不是必须的**。当前文档采用「直接在 Ubuntu 上装 Git/Rust/PostgreSQL + systemd」的方式，足够完成部署和联调。若你希望用 Docker（例如统一环境、多服务编排），可以后续再加 Dockerfile / docker-compose，届时再补文档。

---

## 第一步：GCP 防火墙

若你已经放行了 **80** 和 **443**，无需再开 8080。应用跑在本机 8080，只由 Nginx 在 80/443 对外提供服务。

若尚未放行，在 [GCP Console → VPC 网络 → 防火墙](https://console.cloud.google.com/networking/fireworks/list) 添加入站规则：**tcp 80**、**tcp 443**。

---

## 第二步：在服务器上安装基础环境

SSH 登录到服务器后，依次执行。

### 2.1 更新系统并安装 Git、编译依赖

```bash
sudo apt update
sudo apt install -y git pkg-config libssl-dev build-essential
git --version
```

### 2.2 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustc --version
cargo --version
```

（若重新登录后找不到 `cargo`，再执行一次 `source $HOME/.cargo/env`，或把该行加到 `~/.bashrc`。）

### 2.3 安装 PostgreSQL

```bash
sudo apt install -y postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

---

## 第三步：克隆代码并编译

### 3.1 只拉取服务端相关代码（稀疏检出）

仓库是 monorepo（含客户端 + 服务端），部署时只需服务端。用 **Git sparse-checkout** 只检出构建 admiral-server 需要的路径，不拉取客户端代码。

```bash
cd ~
git clone --no-checkout https://github.com/caobenjamin/Admiral.git
cd Admiral
git sparse-checkout set --no-cone Cargo.toml Cargo.lock crates/admiral-types crates/admiral-server
git checkout main
```

说明：
- `--no-checkout`：先拉取仓库但不检出文件，再按需检出。
- `sparse-checkout set`：只检出工作区根目录的 `Cargo.toml`、`Cargo.lock` 以及 `crates/admiral-types`、`crates/admiral-server`（服务端依赖的共享类型 + 服务端本身），**不会**检出 `crates/admiral-client` 等。

若是私有仓库，需先配置认证（HTTPS 用 Personal Access Token，或配置 SSH 后改用 `git@github.com:caobenjamin/Admiral.git`）。

### 3.2 编译 release

```bash
cargo build --release -p admiral-server
```

编译完成后二进制在：`~/Admiral/target/release/admiral-server`。

---

## 第四步：配置数据库

### 4.1 创建数据库用户和库

用 postgres 用户执行：

```bash
sudo -u postgres psql -c "CREATE USER admiral WITH PASSWORD 'CHANGE_ME_DATABASE_PASSWORD';"
sudo -u postgres psql -c "CREATE DATABASE admiral OWNER admiral;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE admiral TO admiral;"
```

请把 `这里填一个强密码` 换成你自己的密码，后面 `.env` 里要用同一密码。

### 4.2 验证

```bash
sudo -u postgres psql -c "\du"
sudo -u postgres psql -c "\l"
```

应能看到用户 `admiral` 和数据库 `admiral`。

---

## 第五步：配置环境变量与 JWT 密钥

### 5.1 创建运行目录下的 keys 与 .env

服务端会从**当前工作目录**读 `./keys/` 和 `.env`，建议在项目根目录运行并在此创建配置：

```bash
cd ~/Admiral
mkdir -p keys
```

### 5.2 生成 JWT 密钥对

```bash
cd ~/Admiral
openssl genpkey -algorithm RSA -out keys/private.pem -pkeyopt rsa_keygen_bits:2048
openssl rsa -in keys/private.pem -pubout -out keys/public.pem
chmod 600 keys/private.pem keys/public.pem
```

### 5.3 生成 Token 加密密钥（可选，用于 OAuth token 加密）

```bash
openssl rand -hex 32
```

记下输出，填到下面 `.env` 的 `ADMIRAL_TOKEN_ENCRYPTION_KEY`。

### 5.4 创建 .env 文件

**不要**把本地的 `.env` 直接复制到服务器（含敏感信息）。在服务器上新建：

```bash
nano ~/Admiral/.env
```

按下面模板填写（密码、API Key 等换成你自己的；Bilibili 回调地址用你的域名）：

```env
# 数据库（与第四步创建的用户/库/密码一致）
ADMIRAL_DATABASE_URL=postgres://admiral:BINBin_1022@localhost:5432/admiral

# JWT 密钥路径（相对项目根）
ADMIRAL_JWT_PRIVATE_KEY_PATH=./keys/private.pem
ADMIRAL_JWT_PUBLIC_KEY_PATH=./keys/public.pem

# Token 加密密钥（5.3 步生成的 32 字节 hex）
ADMIRAL_TOKEN_ENCRYPTION_KEY=REDACTED_SECRET

# Bilibili OAuth（与你在 B 站开放平台配置的回调一致；用域名）
ADMIRAL_BILI_APP_ID=YOUR_BILIBILI_APP_ID
ADMIRAL_BILI_APP_SECRET=REDACTED_SECRET
ADMIRAL_BILI_CALLBACK_URL=https://admiral.example.com/api/v1/oauth/bilibili/callback

# LLM（当前仅支持 DeepSeek）
ADMIRAL_DEEPSEEK_API_KEY=REDACTED_SECRET

# 服务与日志（用 Nginx 反代时建议只监听本机，不暴露 8080）
ADMIRAL_SERVER_HOST=127.0.0.1
ADMIRAL_SERVER_PORT=8080
ADMIRAL_LOG_LEVEL=info
ADMIRAL_RUN_MODE=production
```

保存后限制权限：

```bash
chmod 600 ~/Admiral/.env
```

**说明**：上面回调地址已用 `https://admiral.example.com`（无端口），需在完成下方「第八步：Nginx 与 HTTPS」后生效；B 站开放平台里填的回调 URL 需与此一致。

---

## 第六步：检查运行状况（前台试跑）

### 6.1 前台启动

```bash
cd ~/Admiral
./target/release/admiral-server
```

应看到类似日志：`listening on 0.0.0.0:8080`。**首次启动会自动执行数据库迁移**。

### 6.2 健康检查

**在服务器上**（另开一个 SSH 终端）：

```bash
curl -s http://127.0.0.1:8080/health
```

应返回 `ok`（或服务端定义的 health 响应）。

**在本机**：若尚未配置 Nginx，需先放行 8080 才能用 `http://admiral.example.com:8080/health` 测试；配置 Nginx 与 HTTPS 后，直接用 `https://admiral.example.com/health` 即可。

确认本机 `curl -s http://127.0.0.1:8080/health` 返回 `ok` 后，用 `Ctrl+C` 停止前台进程，继续后面的 systemd 与 Nginx 步骤。

---

## 第七步：在云端常驻运行（systemd）

用 systemd 让服务在后台常驻、开机自启。

### 7.1 创建 systemd 单元文件

```bash
sudo vim /etc/systemd/system/admiral-server.service
```

内容（注意把 `你的用户名` 换成实际登录用户名，例如 `ubuntu`）：

```ini
[Unit]
Description=Admiral Server
After=network.target postgresql.service

[Service]
Type=simple
User=ben
Group=ben
WorkingDirectory=/home/ben/Admiral
EnvironmentFile=/home/ben/Admiral/.env
ExecStart=/home/ben/Admiral/target/release/admiral-server
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

保存退出。

### 7.2 启用并启动服务

```bash
sudo systemctl daemon-reload
sudo systemctl enable admiral-server
sudo systemctl start admiral-server
sudo systemctl status admiral-server
```

状态应为 `active (running)`。

### 7.3 常用命令

```bash
sudo systemctl status admiral-server   # 查看状态
sudo systemctl restart admiral-server  # 重启（例如更新代码后）
sudo journalctl -u admiral-server -f   # 看日志
```

---

## 第八步：Nginx 与 HTTPS（80/443）

让 Nginx 监听 80/443，用 Let's Encrypt 申请证书，并把请求反代到本机 8080。完成后对外仅用 `https://admiral.example.com` 访问，无需带端口。

### 8.1 安装 Nginx 与 Certbot

```bash
sudo apt install -y nginx certbot python3-certbot-nginx
```

### 8.2 先为域名准备一个 Nginx 站点（HTTP，用于 Certbot 验证）

```bash
sudo vim /etc/nginx/sites-available/admiral.example.com
```

内容（先只写 HTTP，Certbot 会自动改 443）：

```nginx
server {
    listen 80;
    server_name admiral.example.com;
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

保存后启用并检查配置：

```bash
sudo ln -sf /etc/nginx/sites-available/admiral.example.com /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### 8.3 申请 HTTPS 证书（Let's Encrypt）

确保 **admiral-server 已在运行**（systemd 已 start），且 **admiral.example.com 已解析到本机**，再执行：

```bash
sudo certbot --nginx -d admiral.example.com
```

按提示输入邮箱、同意条款。Certbot 会自动修改 Nginx 配置、加上 443 和 SSL，并配置自动续期。

### 8.4 验证

```bash
curl -s https://admiral.example.com/health
```

应返回 `ok`。B 站回调地址使用 `https://admiral.example.com/api/v1/oauth/bilibili/callback` 即可（与 .env 中一致）。

---

## 第九步：再次验证

- 健康检查：`curl -s https://admiral.example.com/health` 返回正常。
- 客户端联调：本地客户端设置 `ADMIRAL_API_BASE_URL=https://admiral.example.com` 连接远端服务端。

### 客户端如何指定服务端地址（持久化配置）

客户端支持**持久化配置文件**，无需每次设环境变量。

- **配置文件路径**（按系统自动选择）：
  - macOS：`~/Library/Application Support/admiral/client.json`
  - Linux：`~/.config/admiral/client.json`
  - Windows：`%APPDATA%\admiral\client.json`
- **格式示例**（新建或编辑该文件）：
  ```json
  {
    "api_base_url": "https://admiral.example.com",
    "credential_path": "credentials.json"
  }
  ```
- **优先级**：默认值 → 上述配置文件 → 环境变量 `ADMIRAL_API_BASE_URL`（环境变量覆盖文件）。
- 项目内示例：`crates/admiral-client/client.json.example`。首次使用可复制为上述路径下的 `client.json` 并改 `api_base_url`。

---

## 之后更新代码的流程

```bash
cd ~/Admiral
git pull
cargo build --release -p admiral-server
sudo systemctl restart admiral-server
```

---

## 简要检查清单

- [ ] GCP 防火墙已放行 tcp 80、443（无需放行 8080）
- [ ] 服务器已安装：git、Rust、pkg-config、libssl-dev、PostgreSQL
- [ ] 已克隆 Admiral、`cargo build --release -p admiral-server` 成功
- [ ] 已创建 PostgreSQL 用户/库，并在 .env 中配置 `ADMIRAL_DATABASE_URL`
- [ ] 已生成 `keys/private.pem`、`keys/public.pem`，并在 .env 中配置路径与 `ADMIRAL_TOKEN_ENCRYPTION_KEY`
- [ ] `.env` 中 `ADMIRAL_SERVER_HOST=127.0.0.1`，`ADMIRAL_BILI_CALLBACK_URL=https://admiral.example.com/api/v1/oauth/bilibili/callback`
- [ ] 前台运行过一次且本机 `curl http://127.0.0.1:8080/health` 返回正常
- [ ] systemd 已配置并 `enable`、`start`，`status` 为 active
- [ ] Nginx 已安装并配置反代到 127.0.0.1:8080，Certbot 已为 admiral.example.com 申请 HTTPS
- [ ] `curl https://admiral.example.com/health` 返回正常

完成以上步骤后，服务端在云端仅通过 **https://admiral.example.com** 对外提供 HTTPS 访问，8080 不对外暴露。Docker 非必需，需要时可再增加 Dockerfile/docker-compose。
