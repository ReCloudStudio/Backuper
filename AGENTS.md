# AGENTS.md — Backuper

## 项目定位

- Rust 服务端备份工具，目标是在服务器上备份普通目录、MySQL、PostgreSQL 等。
- 支持以守护进程运行，并在此基础上提供定时备份与手动触发。
- 提供 WebUI，支持配置化规则引擎。
- 支持通过 Webhook、Discord Bot、Telegram Bot 等方式发送备份状态。
- 支持多种存储后端：本地文件系统、SSH 远程服务器、对象存储等。

## 架构

- Rust workspace，包含 4 个 crate：
  - `crates/backuper-core`：配置解析、领域模型、Source / StorageBackend / Notifier trait、错误类型。
  - `crates/backuper-engine`：备份源与存储后端实现、调度器、retention、通知组装、执行器。
  - `crates/backuper-daemon`：`backuperd` 二进制，守护进程、PID、信号处理、HTTP API、静态资源服务。
  - `crates/backuper-cli`：`backuperctl` 二进制，用于操控守护进程。
- 前端：`webui/`，Nuxt 4 + `@nuxt/ui` latest，开发时通过 proxy 访问 API，release 由后端托管构建产物。
- 状态：`sqlx` + SQLite，减少外部依赖。
- 备份源：
  - `directory`：tar + zstd。
  - `postgres`：调用 `pg_dump`。
  - `mysql`：调用 `mysqldump`。
- 存储后端：
  - `local`：本地文件系统。
  - `ssh`：`russh`/`russh-sftp` 内置，失败降级 `rsync`，再失败降级 `scp`。
  - `s3`：S3-compatible（后续实现）。
- Retention：同时支持保留最近 N 份和保留最近 N 天。

## 开发约定

- 代码注释与用户可见字符串使用简体中文。
- 不使用 emoji，除非用户明确要求。
- 当前阶段使用 cargo / Bun 直接管理工具链（Nix 延后）。
- 敏感信息按全局 AGENTS.md 存放到 `secrets/`，并通过 sops 加密。

## 二进制

- `backuperd`：守护进程，读取配置、执行定时备份、暴露 HTTP API。
- `backuperctl`：CLI 工具，用于启动、停止、重载、手动触发、查看状态等。

常用命令：

```bash
# 守护进程
cargo run --bin backuperd -- --config backuper.toml

# CLI
cargo run --bin backuperctl -- start
cargo run --bin backuperctl -- stop
cargo run --bin backuperctl -- reload
cargo run --bin backuperctl -- run <rule-id>
cargo run --bin backuperctl -- status
cargo run --bin backuperctl -- configtest
```

## WebUI

```bash
cd webui
bun install
bun run dev
```

## 测试 / Lint / 格式化

```bash
cargo test
cargo clippy --workspace --all-targets
cargo fmt --check
```

## 当前限制 / 后续补充

- 已实现：workspace 骨架、配置解析、directory / PostgreSQL / MySQL 备份源、local / SSH 存储后端、retention 清理、cron 调度、守护进程生命周期、HTTP API、CLI 交互、SQLite 任务记录、Webhook / Discord / Telegram 通知、systemd 服务模板与安装脚本。
- 后续应补充：
  - CI / pre-commit 配置。
  - 集成测试 fixture（MySQL / PostgreSQL / SSH）。
  - 对象存储后端（S3-compatible）。
  - WebUI 鉴权与 release 托管。
