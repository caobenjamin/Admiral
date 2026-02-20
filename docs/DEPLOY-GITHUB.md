# 使用 GitHub 克隆方式部署 Admiral Server（GCP Ubuntu）

从零到在远端服务器上运行 `admiral-server` 的完整步骤。

**联调模式**：本仓库采用「服务端在远端运行、本地仅做代码编辑」的方式。服务端部署并运行在 GCP 等远端服务器，本地通过 Git 推送代码后，在服务器上 pull、编译、重启。

---

## 第一步：在本地把代码推送到 GitHub

### 1.1 初始化 Git 并提交（若尚未做过）

在项目根目录执行：

```bash
cd /Users/your-user/Documents/code/Admiral

# 初始化仓库
git init

# 添加所有文件（.gitignore 会排除 target、node_modules、.env 等）
git add .
git commit -m "Initial commit: Admiral server and client"
```

### 1.2 在 GitHub 上创建仓库

1. 打开 [https://github.com/new](https://github.com/new)
2. Repository name 填：`Admiral`（或你喜欢的名字）
3. 选择 **Private** 或 **Public**
4. **不要**勾选 “Add a README file”
5. 点击 **Create repository**

### 1.3 添加远程并推送

把下面命令里的 `你的GitHub用户名` 换成你的实际用户名，仓库名若不同也改一下：

```bash
git remote add origin https://github.com/你的GitHub用户名/Admiral.git
git branch -M main
git push -u origin main
```

若使用 SSH：

```bash
git remote add origin git@github.com:你的GitHub用户名/Admiral.git
git branch -M main
git push -u origin main
```

---

## 第二步：准备 GCP Ubuntu 服务器

### 2.1 确认能 SSH 登录

在本地执行（把 `你的用户名` 和 `服务器IP` 换成实际值）：

```bash
ssh 你的用户名@服务器IP
```

- GCP 默认用户名通常是你在创建 VM 时选的，或 `ubuntu` 等。
- 若使用 GCP 生成的 SSH 密钥，用：  
  `gcloud compute ssh 实例名 --zone=你的zone`

### 2.2 在服务器上安装 Git

登录到服务器后执行：

```bash
sudo apt update
sudo apt install -y git
git --version
```

### 2.3 在服务器上安装 Rust（用于编译 admiral-server）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustc --version
cargo --version
```

若当前 shell 没有自动加载 cargo，以后每次登录后执行一次：

```bash
source $HOME/.cargo/env
```

---

## 第三步：在服务器上克隆并编译

### 3.1 克隆仓库

**公开仓库：**

```bash
cd ~
git clone https://github.com/你的GitHub用户名/Admiral.git
cd Admiral
```

**私有仓库（需先在服务器上配置认证）：**

- **方式 A：HTTPS + Personal Access Token**
  1. GitHub → Settings → Developer settings → Personal access tokens → 生成 token（勾选 `repo`）
  2. 克隆时用：  
     `git clone https://你的用户名:你的token@github.com/你的用户名/Admiral.git`

- **方式 B：SSH Key（推荐）**
  1. 在服务器上生成密钥：  
     `ssh-keygen -t ed25519 -C "server" -f ~/.ssh/id_ed25519 -N ""`
  2. 查看公钥：  
     `cat ~/.ssh/id_ed25519.pub`
  3. 在 GitHub → Settings → SSH and GPG keys 里添加该公钥
  4. 克隆：  
     `git clone git@github.com:你的用户名/Admiral.git`

### 3.2 安装编译依赖（Ubuntu）

admiral-server 使用 SeaORM + PostgreSQL，需要开发库：

```bash
sudo apt install -y pkg-config libssl-dev
```

若使用 PostgreSQL（推荐，与 SeaORM 一致）：

```bash
sudo apt install -y postgresql postgresql-contrib
# 如需从本机连接，再配置 pg_hba.conf 和监听地址
```

### 3.3 编译 release 版本

```bash
cd ~/Admiral
cargo build --release -p admiral-server
```

编译完成后，二进制位置：

```text
~/Admiral/target/release/admiral-server
```

---

## 第四步：在服务器上配置并运行

### 4.1 环境变量与配置

服务端至少需要数据库连接。在服务器上创建环境变量（不要提交到 Git）：

```bash
cd ~/Admiral

# 创建 .env（或只导出到当前 shell）
cat << 'EOF' > .env
ADMIRAL_DATABASE_URL=postgres://用户名:密码@localhost:5432/数据库名
# 其他变量可从你本地的 .env 复制过来，注意改成服务器上的值
EOF
chmod 600 .env
```

也可复制你本地的 `crates/admiral-server/config/` 到服务器同路径，并按服务器环境修改。

### 4.2 启动服务（数据库迁移会在首次启动时自动执行）

前台运行（测试用）：

```bash
cd ~/Admiral
./target/release/admiral-server
```

后台运行（使用 nohup 或 systemd，示例 nohup）：

```bash
cd ~/Admiral
nohup ./target/release/admiral-server > server.log 2>&1 &
```

### 4.3 开放端口（GCP 防火墙）

若服务监听 8080（以你 config 为准）：

1. GCP Console → VPC 网络 → 防火墙
2. 创建规则：允许入站 tcp:8080，目标标签指向你的 VM，或直接选“所有实例”

---

## 第五步：之后更新代码（联调时常用）

在服务器上：

```bash
cd ~/Admiral
git pull
cargo build --release -p admiral-server
# 重启服务（若用 nohup：先 kill 再 nohup 启动；若用 systemd：sudo systemctl restart admiral-server）
```

---

## 简要检查清单

- [ ] 本地：`git init`、`git add .`、`git commit`、GitHub 建仓、`git remote add`、`git push`
- [ ] 服务器：能 SSH 登录、已装 `git`、已装 `rustup`/Rust、已装 `pkg-config` 和 `libssl-dev`
- [ ] 服务器：`git clone`、`cargo build --release -p admiral-server`
- [ ] 服务器：配置 `.env`（至少 `ADMIRAL_DATABASE_URL`）、运行迁移、启动 `admiral-server`
- [ ] GCP：防火墙放行服务监听端口

按上述步骤做完，即可在 GCP Ubuntu 上通过 GitHub 克隆并运行 Admiral Server。本地客户端设置 `ADMIRAL_API_BASE_URL` 指向该服务器地址即可进行联调。
