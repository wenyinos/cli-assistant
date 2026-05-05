# cli-assistant 编译、测试与运行指南

# Build, Test & Run Guide

---

## 目录 / Table of Contents

- [环境要求 / Prerequisites](#环境要求--prerequisites)
- [编译 / Build](#编译--build)
- [配置 / Configuration](#配置--configuration)
- [运行 / Run](#运行--run)
- [测试 / Test](#测试--test)
- [故障排查 / Troubleshooting](#故障排查--troubleshooting)

---

## 环境要求 / Prerequisites

### 系统 / System

| 项目 / Item | 要求 / Requirement |
|---|---|
| 操作系统 / OS | Linux (RHEL 9/10, Fedora, etc.) |
| Rust 工具链 / Rust toolchain | 1.75+ (edition 2021) |
| D-Bus | system bus 可用 / system bus available |
| SQLite | 3.x (运行时自动创建 / auto-created at runtime) |

### 安装 Rust / Install Rust

```bash
# 安装 rustup / Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 确认版本 / Verify version
rustc --version
cargo --version
```

---

## 编译 / Build

### 快速类型检查 / Quick Type Check

不生成二进制文件，速度最快。/ Fastest — no binary output.

```bash
cd /home/zemi/MyDev/cli-assistant
cargo check
```

### Debug 构建 / Debug Build

包含调试信息，未优化。/ Includes debug symbols, unoptimized.

```bash
cargo build
```

产物位置 / Output:
- `target/debug/c` — 客户端 / client binary
- `target/debug/clad` — 守护进程 / daemon binary

### Release 构建 / Release Build

优化后的生产版本。/ Optimized for production.

```bash
cargo build --release
```

产物位置 / Output:
- `target/release/c`
- `target/release/clad`

### 只编译单个 crate / Build a Single Crate

```bash
cargo build -p cla-common    # 共享库 / shared library
cargo build -p cla-dbus      # D-Bus 接口 / D-Bus interfaces
cargo build -p cla-client    # 客户端 / client (bin: c)
cargo build -p cla-daemon    # 守护进程 / daemon (bin: clad)
```

---

## 配置 / Configuration

### 配置文件路径 / Config File Path

```
/etc/cli-assistant/config.toml
```

### 创建配置 / Create Config

```bash
# 创建目录 / Create directory
sudo mkdir -p /etc/cli-assistant

# 复制模板 / Copy template
sudo cp config/config.toml /etc/cli-assistant/config.toml

# 编辑配置 / Edit config
sudo vim /etc/cli-assistant/config.toml
```

### 配置示例 / Config Example

```toml
[backend]
endpoint  = "https://api.openai.com"
model     = "gpt-4"
api_key   = "sk-your-api-key-here"
prompt    = "You are a helpful assistant for Linux system administration."
max_tokens   = 4096
temperature  = 0.7
timeout      = 60

[database]
path = "~/.local/share/cli-assistant/cla.db"

[history]
enabled = true

[logging]
level = "INFO"
```

### 配置字段说明 / Config Fields

| 字段 / Field | 类型 / Type | 默认值 / Default | 说明 / Description |
|---|---|---|---|
| `backend.endpoint` | string | `https://api.openai.com` | API 基础 URL / API base URL |
| `backend.model` | string | `gpt-4` | 模型名称 / Model name |
| `backend.api_key` | string | `""` | API 密钥 / API key |
| `backend.prompt` | string | `"You are a helpful..."` | 系统提示词 / System prompt |
| `backend.max_tokens` | u32 | `4096` | 最大输出 token / Max output tokens |
| `backend.temperature` | f32 | `0.7` | 采样温度 (0.0–2.0) / Sampling temperature |
| `backend.timeout` | u64 | `60` | 请求超时(秒) / Request timeout (sec) |
| `database.path` | string | `~/.local/.../cla.db` | SQLite 文件路径 / SQLite file path |
| `history.enabled` | bool | `true` | 是否记录历史 / Enable history |
| `logging.level` | string | `"INFO"` | 日志级别 / Log level |

### 环境变量 / Environment Variables

| 变量 / Variable | 作用 / Purpose |
|---|---|
| `CL_API_KEY` | 覆盖配置中的 `api_key` / Overrides `api_key` in config |
| `RUST_LOG` | 日志级别 (`debug`, `info`, `warn`, `error`) / Log level |
| `NO_COLOR` | 禁用 ANSI 颜色 / Disable ANSI colors |

---

## 运行 / Run

### 1. 启动守护进程 / Start Daemon

守护进程通过 D-Bus 系统总线注册服务。/ Daemon registers on D-Bus system bus.

```bash
# 前台运行 (调试) / Foreground (debug)
sudo RUST_LOG=debug ./target/debug/clad

# 后台运行 / Background
sudo ./target/release/clad &

# 查看日志 / View logs
journalctl -u clad -f   # 如果配置了 systemd / if systemd configured
```

> **注意 / Note:** 守护进程需要 root 权限或 D-Bus policy 配置。
> The daemon requires root or a D-Bus policy configuration.

### 2. 使用客户端 / Use Client

```bash
# 直接提问 / Ask a question
./target/debug/c "How do I check disk space?"

# 使用 stdin / Use stdin
echo "What is SELinux?" | ./target/debug/c

# 指定模型 (覆盖配置) / Specify model (override config)
./target/debug/c -m gpt-3.5-turbo "Explain systemd"

# 查看帮助 / Show help
./target/debug/c --help
```

### 3. 子命令 / Subcommands

```bash
# 聊天 (默认) / Chat (default)
./target/debug/c chat "How to list running services?"

# 交互模式 / Interactive mode
./target/debug/c chat --interactive

# 查看历史 / View history
./target/debug/c history --all
./target/debug/c history --filter "systemd"

# 清除历史 / Clear history
./target/debug/c history --clear
./target/debug/c history --clear-all

# 反馈 / Feedback
./target/debug/c feedback

# Shell 集成 / Shell integration
./target/debug/c shell --enable-interactive
./target/debug/c shell --disable-interactive
```

### 4. 完整端到端测试 / Full E2E Test

```bash
# 终端 1: 启动守护进程 / Terminal 1: Start daemon
sudo RUST_LOG=debug ./target/debug/clad

# 终端 2: 发送请求 / Terminal 2: Send request
./target/debug/c "What is SELinux?"
```

预期输出 / Expected output:

```
⁺₊+ Asking RHEL Lightspeed...
────────────────────────────────────────────────────────────────────────
SELinux (Security-Enhanced Linux) is a security architecture...
────────────────────────────────────────────────────────────────────────
Always review AI-generated content prior to use.
```

---

## 测试 / Test

### 单元测试 / Unit Tests

```bash
# 运行所有测试 / Run all tests
cargo test

# 只运行某个 crate 的测试 / Run tests for a single crate
cargo test -p cla-common
cargo test -p cla-daemon

# 运行特定测试 / Run a specific test
cargo test -p cla-common config::tests::default_config_roundtrips

# 显示测试输出 / Show test output
cargo test -- --nocapture
```

### 测试覆盖范围 / Test Coverage

| Crate | 测试内容 / What's Tested |
|---|---|
| `cla-common` | 配置序列化/反序列化、UUID 生成、文件操作、XDG 路径 |
| `cla-dbus` | D-Bus 错误类型转换 |
| `cla-daemon` | (运行时集成测试 / runtime integration) |
| `cla-client` | (运行时集成测试 / runtime integration) |

### 验证配置加载 / Verify Config Loading

```bash
# 测试配置是否能正确加载 / Test if config loads correctly
RUST_LOG=debug ./target/debug/clad 2>&1 | head -5

# 预期输出 / Expected:
# 2024-... INFO  clad 0.6.0 starting
# 2024-... INFO  Configuration loaded successfully
```

---

## 故障排查 / Troubleshooting

### 常见问题 / Common Issues

| 问题 / Problem | 原因 / Cause | 解决方案 / Solution |
|---|---|---|
| `Connection refused` | 守护进程未运行 / Daemon not running | `sudo ./target/debug/clad` |
| `Permission denied` | D-Bus 权限不足 / D-Bus permission | 用 `sudo` 运行 / Run with `sudo` |
| `No API key configured` | 未设置 API key / No API key set | 编辑配置文件或设置 `CL_API_KEY` / Edit config or set `CL_API_KEY` |
| `config.toml not found` | 配置文件缺失 / Config file missing | `sudo cp config/config.toml /etc/cli-assistant/` |
| `SQLite error` | 数据库路径无权限 / DB path no permission | 检查 `database.path` 目录权限 / Check `database.path` directory permissions |

### 调试模式 / Debug Mode

```bash
# 最详细的日志 / Maximum verbosity
RUST_LOG=trace ./target/debug/clad

# 只看错误 / Errors only
RUST_LOG=error ./target/debug/clad

# 只看特定模块 / Specific module only
RUST_LOG=cla_daemon::http=debug ./target/debug/clad
```

### 手动测试 API 连通性 / Test API Connectivity

```bash
# 用 curl 测试 OpenAI 兼容接口 / Test OpenAI-compatible endpoint with curl
curl -X POST https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-your-key" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 100
  }'
```

---

## 快速参考 / Quick Reference

```bash
# 编译 / Build
cargo check                          # 类型检查 / Type check
cargo build                          # Debug 构建 / Debug build
cargo build --release                # Release 构建 / Release build

# 运行 / Run
sudo RUST_LOG=debug ./target/debug/clad      # 启动 daemon / Start daemon
./target/debug/c "your question"              # 发送请求 / Send request

# 测试 / Test
cargo test                           # 运行所有测试 / Run all tests
cargo test -p cla-common             # 单个 crate / Single crate

# 清理 / Clean
cargo clean                          # 清除构建产物 / Remove build artifacts
```
