# Backuper

Backuper 是一个面向服务器的备份工具，支持定时或手动备份目录、MySQL、PostgreSQL 等数据源，并将归档上传至本地、SSH 远程或 S3 兼容对象存储。

## 功能

- **备份源**：目录（tar + zstd）、PostgreSQL（pg_dump）、MySQL（mysqldump）。
- **存储后端**：本地文件系统、SSH/SFTP（内置 `russh`，降级 `rsync`/`scp`）、S3 兼容对象存储（R2 / MinIO 等）。
- **调度与保留**：基于 cron 的定时任务，同时支持保留最近 N 份和最近 N 天。
- **通知**：备份成功后发送 Webhook、Discord Bot 或 Telegram Bot 通知。
- **WebUI**：基于 Nuxt 4 + `@nuxt/ui` 的仪表盘，支持登录鉴权、查看规则、手动触发和任务历史。
- **CLI**：`backuperctl` 用于启动、停止、重载、查看状态与手动触发备份。
- **守护进程**：`backuperd` 提供 HTTP API、cron 调度、SQLite 状态记录与 systemd 服务模板。

## 架构

```
Backuper/
├── crates/backuper-core     # 配置、领域模型、Source / StorageBackend / Notifier trait
├── crates/backuper-engine   # 备份源实现、存储后端、调度器、retention、通知组装
├── crates/backuper-daemon   # backuperd 守护进程与 HTTP API
├── crates/backuper-cli      # backuperctl 命令行工具
└── webui/                   # Nuxt 4 WebUI
```

## 快速开始

### 依赖

- Rust（见 `rust-toolchain.toml`）
- Bun 1.3.x（用于 WebUI）
- Docker（可选，用于运行集成测试）
- systemd（可选，用于安装服务）

### 构建

```bash
cargo build --release

cd webui
bun install
bun run build
```

release 产物位于 `target/release/backuperd`、`target/release/backuperctl` 和 `webui/.output/public`。

### 安装并运行服务

```bash
sudo ./scripts/install.sh
sudo systemctl enable --now backuper
```

安装脚本会创建 `backuper` 用户、安装二进制文件、构建并安装 WebUI 静态资源、安装 systemd 单元。

### 配置

参考 `examples/backuper.toml`：

```toml
[global]
data_dir = "/var/lib/backuper"
listen = "127.0.0.1:8080"
api_token = "CHANGE_ME"

[[rule]]
id = "docs"
schedule = "0 2 * * *"
storage = "local_backups"

[rule.source]
type = "directory"
path = "/srv/docs"

[rule.retention]
keep_last = 7
keep_days = 30

[[storage]]
id = "local_backups"
type = "local"
path = "/backup/backuper"
```

启用 `api_token` 后，`/api/*` 端点需要 `Authorization: Bearer <token>`。

## CLI

```bash
backuperctl start          # 启动守护进程
backuperctl stop           # 停止守护进程
backuperctl reload         # 重载配置
backuperctl run <rule-id>  # 手动触发规则
backuperctl status         # 查看状态
backuperctl configtest     # 检查配置语法
```

## HTTP API

守护进程默认监听 `127.0.0.1:8080`。

| 路径 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/api/login` | POST | WebUI 登录验证 token |
| `/api/status` | GET | 状态与最近任务 |
| `/api/rules` | GET | 规则列表 |
| `/api/run/{rule_id}` | POST | 手动触发规则 |
| `/api/reload` | POST | 重载配置 |

未配置 `api_token` 时，`/api/*` 公开访问；配置后需携带 `Authorization: Bearer <token>`。

## 测试

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
```

Docker 集成测试默认被 `#[ignore]` 跳过，显式运行：

```bash
cargo test --workspace --all-targets -- --ignored
```

## 贡献

提交前请运行：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace --all-targets
```

## 许可证

[MIT](LICENSE)
