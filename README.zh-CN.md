[English](README.md) | [中文](README.zh-CN.md)

# cli-assistant

一个快速、轻量的 Linux 系统管理命令行助手 — 支持任意 OpenAI 兼容 API。

> **灵感来源** [command-line-assistant](https://github.com/rhel-lightspeed/command-line-assistant)（RHEL Lightspeed 团队）。本项目使用 Rust 重写，提升了可移植性、性能和灵活性。

## 功能特性

- **自然语言提问** — 直接在终端用自然语言咨询系统管理问题
- **OpenAI 兼容** — 支持 OpenAI、Azure OpenAI、本地 LLM 或任意 OpenAI 兼容端点
- **配置向导** — `sudo c setup` 交互配置端点、API Key、模型（可从端点 `/models` 列表选择）与回复语言
- **会话与历史** — SQLite 持久化对话历史记录
- **交互模式** — 命令行与全屏 TUI 多轮对话，自动携带近期上下文，历史过长时压缩为摘要
- **Markdown 渲染** — 终端彩色输出，支持代码块、表格、标题
- **语言配置** — 可指定 AI 回复语言（如中文、英文、日文等）
- **D-Bus 守护进程架构** — 客户端/服务端分离，支持系统自动激活与调用方授权
- **终端捕获** — 通过 `c shell --enable-capture` 把最近终端输出作为问题上下文
- **审计日志** — 通过结构化 tracing 输出审计事件

## 安装方式

**x86_64** 与 **aarch64** 的预编译包发布在 [Releases](../../releases) 页面：

| 发行版 | 包格式 | 安装命令 |
|---|---|---|
| Fedora / RHEL | `.rpm` | `sudo dnf install ./cli-assistant-*.rpm` |
| Debian / Ubuntu | `.deb` | `sudo apt install ./cli-assistant_*.deb` |
| Arch Linux | `packaging/PKGBUILD` | `cd packaging && makepkg -s` |

RPM 与 DEB 由 CI 构建；Arch Linux（含 Arch Linux ARM）按该生态惯例用
PKGBUILD 本地构建。

从本仓库自行构建软件包：

```bash
# RPM（需先将源码 tarball 放入 ~/rpmbuild/SOURCES；CI 在打 tag 时自动生成）
rpmbuild -ba packaging/cli-assistant.spec

# Debian / Ubuntu（native 包，原地构建）
dpkg-buildpackage -us -uc -b

# Arch Linux —— 以及 Arch Linux ARM（aarch64 由本地 makepkg 构建）
cd packaging && makepkg -s
```

软件包会安装二进制（`c`, `clad`）、D-Bus 策略与自动激活文件、systemd 服务、
man pages，并创建 `/etc/cli-assistant/` 目录。配置文件本身由设置向导生成：

```bash
# 首次配置 — 向导依次询问端点、API key、模型（可从端点 /models 列表选择）
# 和回复语言，写入 /etc/cli-assistant/config.toml 并自动重启 clad
sudo c setup

# 测试
c "如何检查磁盘空间？"
```

### 从源码编译

```bash
# 编译
cargo build --release

# 配置 — 写入 /etc/cli-assistant/config.toml 并重启 clad
sudo ./target/release/c setup

# 安装 D-Bus 策略文件
sudo cp config/com.cli-assistant.conf /etc/dbus-1/system.d/

# 运行
sudo ./target/release/clad &          # 启动守护进程
./target/release/c "如何检查磁盘空间？"  # 提问
```

详细的编译、测试和运行说明请参阅 **[AGENTS.md](AGENTS.md)**。

### 卸载

```bash
sudo dnf remove cli-assistant cli-assistant-selinux   # Fedora / RHEL
sudo apt remove cli-assistant                          # Debian / Ubuntu
sudo pacman -R cli-assistant                           # Arch Linux
```

配置（`/etc/cli-assistant/`）与数据（`/var/lib/cli-assistant/`）会保留，
需要彻底清理请手动删除。

### 服务管理

```bash
sudo systemctl status clad     # 查看服务状态
sudo systemctl restart clad    # 重启（手工修改配置后必须执行）
sudo systemctl stop clad       # 停止守护进程
sudo systemctl start clad      # 启动守护进程
sudo systemctl enable clad     # 设为开机自启（由软件包完成）
sudo systemctl disable clad    # 取消开机自启
journalctl -u clad -f          # 查看实时日志
```

## 配置说明

配置文件路径：`/etc/cli-assistant/config.toml`，由 `sudo c setup` 向导生成。
手工编辑后需重启守护进程（`sudo systemctl restart clad`）。

```toml
[backend]
endpoint  = "https://api.deepseek.com/v1"   # 任意 OpenAI 兼容端点
model     = "deepseek-v4-flash"
api_key   = "sk-..."
prompt    = "You are a command-line assistant for Linux system administration. Answer concisely and accurately, and prefer standard, widely available tools. Keep commands copy-pasteable; before any destructive or irreversible step, explain what it does and call out the risk. If a request is ambiguous, state your assumption briefly and answer the most likely intent."
language  = "zh-CN"                     # 回复语言（留空则自动判断）
max_tokens     = 32768
context_length = 256000                 # 模型上下文窗口（用于触发自动压缩）
temperature  = 0.3
timeout      = 120

[database]
path = "/var/lib/cli-assistant/cla.db"

[history]
enabled = true

[logging]
level = "INFO"

[logging.audit]
enabled = true
```

API key 也可通过环境变量 `CL_API_KEY` 设置（优先级高于配置文件）。
daemon 在所有发行版上只读取 `/etc/cli-assistant/config.toml` 一个配置文件。

## 使用方法

```bash
sudo c setup                        # 首次配置向导：端点、API key、模型、回复语言
c "问题"                            # 提问（默认使用 chat 子命令）
c chat "问题"                       # 同上
c chat --interactive                # 进入交互对话模式
c chat --tui                        # 进入全屏 TUI 对话模式
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
| **LLM 配置** | 后端硬编码 | 可配置模型、密钥、提示词、温度、最大 token、上下文长度、语言 |
| **数据库** | SQLAlchemy (SQLite/MySQL/PostgreSQL) | sqlx + SQLite（更简单、更轻量） |
| **IPC** | dasbus (Python D-Bus) | zbus 4.x（原生异步 Rust D-Bus） |
| **HTTP** | requests + urllib3 | reqwest + rustls（异步，无 OpenSSL 依赖） |
| **CLI** | argparse + 装饰器模式 | clap 4 derive（类型安全，自动补全） |
| **渲染** | python-markdown → ANSI | 自研 markdown→ANSI 渲染器 |
| **依赖** | ~10 个 Python 包 | 纯 Rust crate，Cargo 统一管理 |
| **启动速度** | ~200ms (Python 导入) | ~5ms (原生二进制) |
| **Docker/CI** | 有但复杂 | GitHub Actions：fmt/clippy/test，以及 x86_64/aarch64 的 rpm/deb/pacman 构建 |

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
| `cla-client` | CLI 解析、渲染器、配置向导、D-Bus 客户端 |
| `cla-daemon` | D-Bus 服务端、HTTP 客户端、SQLite 存储、历史管理 |

## 许可证

MIT
