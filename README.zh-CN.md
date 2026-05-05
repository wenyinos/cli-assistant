# cli-assistant

一个快速、轻量的 Linux 系统管理命令行助手 — 支持任意 OpenAI 兼容 API。

> **灵感来源** [command-line-assistant](https://github.com/rhel-lightspeed/command-line-assistant)（RHEL Lightspeed 团队）。本项目使用 Rust 重写，提升了可移植性、性能和灵活性。

## 功能特性

- **自然语言提问** — 直接在终端用自然语言咨询系统管理问题
- **OpenAI 兼容** — 支持 OpenAI、Azure OpenAI、本地 LLM 或任意 `/v1/chat/completions` 端点
- **会话与历史** — SQLite 持久化对话历史记录
- **交互模式** — 支持多轮连续对话
- **Markdown 渲染** — 终端彩色输出，支持代码块、表格、标题
- **语言配置** — 可指定 AI 回复语言（如中文、英文、日文等）
- **D-Bus 守护进程架构** — 客户端/服务端分离，支持系统级集成

## 快速开始

```bash
# 编译
cargo build --release

# 配置
sudo mkdir -p /etc/cli-assistant
sudo cp config/config.toml /etc/cli-assistant/config.toml
# 编辑 /etc/cli-assistant/config.toml — 填入 API key 和端点地址

# 安装 D-Bus 策略文件
sudo cp config/com.cli-assistant.conf /etc/dbus-1/system.d/

# 运行
sudo ./target/release/clad &          # 启动守护进程
./target/release/c "如何检查磁盘空间？"  # 提问
```

详细的编译、测试和运行说明请参阅 **[docs/BUILD.md](docs/BUILD.md)**。

## 配置说明

配置文件路径：`/etc/cli-assistant/config.toml`

```toml
[backend]
endpoint  = "https://api.openai.com"   # 任意 OpenAI 兼容端点
model     = "gpt-4"
api_key   = "sk-..."
prompt    = "You are a helpful assistant for Linux system administration."
language  = "zh-CN"                     # 回复语言（留空则自动判断）
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

API key 也可通过环境变量 `CL_API_KEY` 设置（优先级高于配置文件）。

## 使用方法

```bash
c "问题"                            # 提问（默认使用 chat 子命令）
c chat "问题"                       # 同上
c chat --interactive                # 进入交互对话模式
c chat -a /path/to/file "解释这个"   # 附加文件作为上下文
c history --all                     # 查看所有历史记录
c history --filter "关键词"          # 搜索历史记录
c history --clear                   # 清除当前会话历史
c feedback                          # 查看反馈信息
c shell --enable-interactive        # 启用 Ctrl+G 快捷键
```

## 相对原项目的改进

| 方面 | Python 原项目 | 本项目 (Rust) |
|---|---|---|
| **语言** | Python 3.9+ | Rust (edition 2021) |
| **运行时** | CPython + pip 依赖 | 单个静态二进制，无运行时依赖 |
| **API 后端** | 仅 RHEL Lightspeed | 任意 OpenAI 兼容端点 |
| **LLM 配置** | 后端硬编码 | 可配置模型、密钥、提示词、温度、最大 token、语言 |
| **数据库** | SQLAlchemy (SQLite/MySQL/PostgreSQL) | sqlx + SQLite（更简单、更轻量） |
| **IPC** | dasbus (Python D-Bus) | zbus 4.x（原生异步 Rust D-Bus） |
| **HTTP** | requests + urllib3 | reqwest + rustls（异步，无 OpenSSL 依赖） |
| **CLI** | argparse + 装饰器模式 | clap 4 derive（类型安全，自动补全） |
| **渲染** | python-markdown → ANSI | 自研 markdown→ANSI 渲染器 |
| **依赖** | ~10 个 Python 包 | 纯 Rust crate，Cargo 统一管理 |
| **启动速度** | ~200ms (Python 导入) | ~5ms (原生二进制) |
| **Docker/CI** | 有但复杂 | 不需要 — `cargo build` 即可 |

## 架构

```
c (客户端)  ──D-Bus──▶  clad (守护进程)  ──HTTP──▶  LLM API
                           │
                           └── SQLite (历史记录)
```

| Crate | 职责 |
|---|---|
| `cla-common` | 配置、错误处理、会话管理、文件工具、系统信息 |
| `cla-dbus` | D-Bus 接口定义与数据结构 |
| `cla-client` | CLI 解析、渲染器、D-Bus 客户端 |
| `cla-daemon` | D-Bus 服务端、HTTP 客户端、SQLite 存储、历史管理 |

## 许可证

MIT
