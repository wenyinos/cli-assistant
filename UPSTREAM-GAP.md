# 上游差距分析 / Upstream Gap Analysis

本文档对比本 Rust 项目（cli-assistant v0.8.0）与上游 Python 项目
[`rhel-lightspeed/command-line-assistant`](https://github.com/rhel-lightspeed/command-line-assistant)
（本地副本 `/home/zemi/MyWork/command-line-assistant`，commit `da81fdd`，v0.5.1，Apache-2.0），
列出需要新增的功能特性、回归缺陷与工程改进，供后续开发排期参考。

This document compares this Rust project (cli-assistant v0.8.0) against the upstream
Python project (local copy at `/home/zemi/MyWork/command-line-assistant`, commit
`da81fdd`, v0.5.1), listing missing features, regressions, and engineering improvements.

生成日期 / Generated: 2026-08-03

---

## 优先级定义 / Priority

| 级别 / Level | 含义 / Meaning |
|---|---|
| **P0** | 正确性回归 — 上游行为正确，Rust 版行为错误，应尽快修复 / Behavioral regression vs upstream |
| **P1** | 核心功能缺失 — 上游有、Rust 版无，影响实际使用 / Core missing feature |
| **P2** | 值得做 — 完善性 / 工程改进 / Nice-to-have |
| **P3** | 决策记录 — 有意的简化，维持现状 / Deliberate simplification, keep as-is |

## 总览 / Overview

| ID | 项目 / Item | 优先级 | 工作量 | 状态 / Status |
|---|---|---|---|---|
| B1 | 历史记录存错：question 被存成 response / History stores wrong question | P0 | S | 已修复 / Fixed |
| B2 | `history.enabled` 配置未生效 / Config not honored | P0 | S | 已修复 / Fixed |
| A1 | 终端捕获（pty）实现 / Terminal capture (pty) | P1 | L | 已实现 / Implemented |
| A2 | D-Bus 调用方授权校验 / Caller authorization | P1 | M | 已实现 / Implemented |
| A3 | D-Bus 自动激活 / D-Bus activation | P1 | S | 已实现 / Implemented |
| A4 | 审计日志（journald） / Audit logging | P2 | M | 已实现 / Implemented |
| A5 | SELinux 策略 / SELinux policy | P2 | L | 已实现 / Implemented |
| A6 | man pages 与 RPM 打包 / man pages & RPM packaging | P2 | M | 已实现 / Implemented |
| A7 | XDG 配置路径感知 / XDG-aware config lookup | P2 | S | 已实现 / Implemented |
| B3 | 重复且不一致的 session 推导代码 / Duplicate session derivation | P2 | S | 已清理 / Cleaned |
| B4 | 未使用依赖清理 / Unused dependency cleanup | P2 | S | 已清理 / Cleaned |
| C1 | daemon/client 单元测试覆盖 / Unit test coverage | P1 | L | 已实现 / Implemented |
| C2 | CI 增加 test / clippy / CI test & lint | P2 | S | 已添加 / Added |
| D1–D5 | 明确不做的决策 / Deliberate non-goals | P3 | — | 记录在案 / Documented |

---

## A. 缺失功能 / Missing Features

### A1. 终端捕获 / Terminal Capture

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `command_line_assistant/terminal/reader.py`（`TerminalRecorder` 通过 pty 监听，按提示符标记 `\x1b]` 切块，写 `~/.local/state/command-line-assistant/terminal.log` JSONL）、`terminal/parser.py`；`c shell --enable-capture` 安装 bashrc 片段并驱动捕获；捕获运行时交互模式被文件锁阻塞（`commands/chat.py:613-619`）
- **Rust 版现状 / Current state**: `terminal.rs` 使用 `nix::pty::forkpty` + `poll` 实现 PTY 捕获，stdin 会转发给子 shell；按 `\x1b]` 提示符标记切分 command/output 并写 JSONL；`shell --enable-capture` 与交互模式通过文件锁互斥
- **建议实现要点 / Implementation notes**: 已实现；ANSI/OSC 清理、`-w` 反向索引与 recorder 均有单元测试

### A2. D-Bus 调用方授权校验 / Caller Authorization

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `command_line_assistant/dbus/interfaces/authorization.py` — `DBusAuthorizationMixin` 在**每个接口方法**入口校验：`GetConnectionUnixUser(sender)` 必须匹配请求的 euid（`chat.py:47-84`）或推导出的内部 user id（`chat.py:86-126`），fail-closed；通过 `dbus/server.py` 的 `SpecialServerObjectHandler` 把 sender 注入 thread-local
- **Rust 版现状 / Current state**: 新增 `authorization.rs`，所有 chat/history/user 方法通过 `#[zbus(header)]` + `#[zbus(connection)]` 获取 sender，并经 `DBusProxy::GetConnectionUnixUser` 做内部 user id 校验；失败返回 `AccessDenied`
- **建议实现要点 / Implementation notes**: 已实现；Unix user 与内部 user 校验均有纯函数单元测试

### A3. D-Bus 自动激活 / D-Bus Activation

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `data/release/dbus/com.redhat.lightspeed.{chat,history,user}.service`（`[D-BUS Service]`，`SystemdService=clad.service`，`Exec=/bin/false`）— 任意用户请求总线名时 systemd 自动拉起 clad
- **Rust 版现状 / Current state**: 已新增三个 `[D-BUS Service]` 文件并接入 `install.sh`/`uninstall.sh`/release 归档
- **建议实现要点 / Implementation notes**: 已实现；D-Bus policy 同步补充 `receive_sender` 放行

### A4. 审计日志 / Audit Logging

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: 配置 `[logging.audit]`（`config/schemas/logging.py`）→ journald 结构化事件
- **Rust 版现状 / Current state**: `config.toml` 默认启用 `[logging.audit]`；新增 `audit.rs`，在 AskQuestion/CreateChat/WriteHistory/ClearHistory/ClearAllHistory 入口发 `target: "audit"` 结构化事件
- **建议实现要点 / Implementation notes**: 已实现

### A5. SELinux 策略 / SELinux Policy

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `data/release/selinux/{clad.te, clad.if, clad.fc, Makefile}` — 为 `clad` 定制策略模块
- **Rust 版现状 / Current state**: `data/release/selinux/` 提供 `cli_assistant.{te,if,fc}` 与 Makefile
- **建议实现要点 / Implementation notes**: 已实现；编译安装步骤写入 `docs/BUILD.md`

### A6. man pages 与 RPM 打包 / man Pages & RPM Packaging

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `data/release/man/{c.1, clad.8}`；`packaging/command-line-assistant.spec` + `.packit.yaml` / `.gitlab-ci.yml` 下游打包
- **Rust 版现状 / Current state**: 新增 `data/release/man/{c.1,clad.8}` 与 `packaging/cli-assistant.spec`；install/uninstall 与 release 归档已包含
- **建议实现要点 / Implementation notes**: 已实现；spec 构建 Rust 二进制、systemd/D-Bus 激活、man pages 与 SELinux 子包

### A7. XDG 配置路径感知 / XDG-aware Config Lookup

- **状态 / Status**: 已实现 / Implemented
- **上游参考 / Upstream reference**: `config/__init__.py:26-29, 56-82` + `utils/environment.py:70-111` — 支持 `XDG_CONFIG_DIRS`（如 `/etc/xdg/command-line-assistant/config.toml`）多路径查找
- **Rust 版现状 / Current state**: `Config::load()` 现在读取 `$XDG_CONFIG_DIRS/command-line-assistant/config.toml` 多路径、`$XDG_CONFIG_HOME`，并保持 `/etc/cli-assistant/config.toml` 最高优先级
- **建议实现要点 / Implementation notes**: 已实现；候选路径加载逻辑有单元测试

---

## B. 回归缺陷 / Regressions (upstream correct → Rust broken)

### B1. 历史记录存错：question 被存成 response / History Stores the Response as Question

- **状态 / Status**: 已修复 / Fixed — **P0**
- **上游参考 / Upstream reference**: `commands/chat.py:379-412` — `WriteHistory(chat_id, user_id, message, response)` 传入真实问题
- **Rust 版现状 / Current state**: `crates/cla-client/src/main.rs:330-333`（单发）与 `main.rs:384-386`（交互模式）均为
  ```rust
  dbus.write_history(&chat_id, &user_id, &response.message, &response.message)
  ```
  **用户的问题文本从未入库** — `interactions` 表每行是回答内容的两份拷贝；`history --filter` 按问题搜索将永远无法命中
- **修复 / Fix**: 已改为通过 `history_payload` 将实际 `question.message` 与 `response.message` 分开写入；单发与交互模式均已覆盖，并补了回归测试

### B2. `history.enabled` 配置未生效 / `history.enabled` Not Honored

- **状态 / Status**: 已修复 / Fixed — **P0**
- **上游参考 / Upstream reference**: 上游 daemon 检查 `history.enabled`，禁用时抛出 `HistoryNotEnabledError`，客户端 `_submit_question` 将其吞掉（不写历史、不报错）
- **Rust 版现状 / Current state**: `config.rs:136-145` 定义了 `history.enabled`，但 daemon 的 `LocalHistory::write`（`history/local.rs:99-117`）**无条件写入**，配置形同虚设
- **修复 / Fix**: `HistoryInterface` 所有 history 方法入口统一检查 `config.history.enabled`；禁用时返回 `com.redhat.lightspeed.HistoryNotEnabled`（D-Bus 层仍映射为 `fdo::Error::Failed`），客户端对写入错误静默忽略，读取与清理同样被拒绝

### B3. 重复且不一致的 session 推导代码 / Duplicate Session Derivation

- **状态 / Status**: 已清理 / Cleaned — **P2**
- **上游参考 / Upstream reference**: `daemon/session.py:51-64` — `uuid5(uuid(machine-id), euid)`
- **Rust 版现状 / Current state**: daemon 已统一使用 `cla-common::UserSessionManager`，不再维护第二份实现；公共实现按上游 `uuid5(uuid(machine-id), euid)` 语义生成用户 ID
- **修复 / Fix**: 已消除重复实现，并补上游已知值回归测试

### B4. 未使用依赖清理 / Unused Dependency Cleanup

- **状态 / Status**: 已清理 / Cleaned — **P2**
- **上游参考 / Upstream reference**: 上游使用 `python-markdown` 扩展；Rust 版手写渲染器是其刻意替代
- **Rust 版现状 / Current state**: 已从 workspace 与 `cla-client` 清单移除 `termimad`、`crossterm`、`pulldown-cmark`
- **修复 / Fix**: 已完成依赖清理；若未来 `-w` 终端捕获需要 ANSI 剥离或富文本渲染，再按需引入

---

## C. 测试与 CI / Testing & CI

### C1. daemon / client 单元测试覆盖 / Unit Test Coverage

- **状态 / Status**: 已实现 / Implemented — **P1**
- **上游参考 / Upstream reference**: `tests/` 43 个 pytest 文件，覆盖命令解析、配置、DB、HTTP（`responses` mock）、D-Bus 接口、渲染、终端解析等；`tox.ini` + 覆盖率
- **Rust 版现状 / Current state**: 已覆盖参数合成、输入截断、历史写入、D-Bus 授权、history enabled、HTTP payload/status/退避/响应解析、本地 TCP HTTP 成功/4xx/重试路径、Markdown 渲染与终端解析
- **建议 / Recommendation**:
  1. `query.rs` 用 `httpmock` 或 mock reqwest 测 URL 构造、重试/退避、4xx/5xx/传输错误分支
  2. `main.rs` 的 `add_default_command`、`gather_input` 优先级、`MAX_QUESTION_SIZE` 截断、交互循环收敛为可测纯函数
  3. `rendering/markdown.rs` 各语法块（代码框/表格/列表/行内样式）快照测试
  4. 修复 B1/B2 时同步补回归测试

### C2. CI 增加 test / clippy / CI Test & Lint

- **状态 / Status**: 已添加 / Added — **P2**
- **上游参考 / Upstream reference**: pre-commit 自动更新、`make unit-test`（uv + pytest）、tox 矩阵
- **Rust 版现状 / Current state**: 新增 `.github/workflows/ci.yml`，PR/main 触发 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`
- **建议 / Recommendation**: 已实现；本地已通过 fmt/clippy/test

---

## D. 明确不做的决策 / Deliberate Non-goals (keep as-is)

以下差异是**有意的设计选择**，不应视为缺口：

| ID | 差异 / Difference | 决策理由 / Rationale |
|---|---|---|
| D1 | `/infer` 专有协议 + 3scale 错误解析 / Proprietary `/infer` protocol | 改为 OpenAI 兼容 `chat/completions`，支持任意端点（OpenAI/Azure/本地 LLM） |
| D2 | RHSM mTLS 客户端证书认证 / mTLS with RHSM certs | 改为 Bearer API key（`CL_API_KEY` 或配置）；`config.toml` 模板保留 `[backend.auth]` 注释示例供需要时启用 |
| D3 | MySQL / PostgreSQL 多数据库后端 / Multi-backend DB | 仅 SQLite（sqlx），更轻量；上游的 ORM 灵活性非本项目目标 |
| D4 | Python 生态打包（pip / RPM 依赖解析）/ Python packaging | 单静态二进制 + `scripts/install.sh`，零运行时依赖；RPM spec（A6）可选加 |
| D5 | 多轮对话上下文注入 / Multi-turn context injection | 上游同样不注入（单发式 `AskQuestion`）；若要做属于新特性而非对齐上游 |

---

## 建议实施顺序 / Suggested Order

1. **B1 + B2**（P0 正确性，各 ~半天）：修历史存储与配置生效，附带回归测试
2. **C1 起步 + B3/B4**（P2 清理，与测试互为前提）
3. **A3 + A2**（P1，S/M）：先让 daemon 可自动拉起，再补授权校验（两者同属 D-Bus 层）
4. **A1 终端捕获**（P1，L）：独立大块，建议排期里程碑
5. **A4/A5/A6/A7、C2**（P2）按发布节奏填充
