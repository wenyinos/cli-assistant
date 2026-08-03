# 上游差距分析 / Upstream Gap Analysis

本文档对比本 Rust 项目（cli-assistant v0.6.5）与上游 Python 项目
[`rhel-lightspeed/command-line-assistant`](https://github.com/rhel-lightspeed/command-line-assistant)
（本地副本 `/home/zemi/MyWork/command-line-assistant`，commit `da81fdd`，v0.5.1，Apache-2.0），
列出需要新增的功能特性、回归缺陷与工程改进，供后续开发排期参考。

This document compares this Rust project (cli-assistant v0.6.5) against the upstream
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
| B1 | 历史记录存错：question 被存成 response / History stores wrong question | P0 | S | 未修复 / Open |
| B2 | `history.enabled` 配置未生效 / Config not honored | P0 | S | 未修复 / Open |
| A1 | 终端捕获（pty）实现 / Terminal capture (pty) | P1 | L | 未实现 / Missing |
| A2 | D-Bus 调用方授权校验 / Caller authorization | P1 | M | 未实现 / Missing |
| A3 | D-Bus 自动激活 / D-Bus activation | P1 | S | 未实现 / Missing |
| A4 | 审计日志（journald） / Audit logging | P2 | M | 未实现 / Missing |
| A5 | SELinux 策略 / SELinux policy | P2 | L | 未实现 / Missing |
| A6 | man pages 与 RPM 打包 / man pages & RPM packaging | P2 | M | 未实现 / Missing |
| A7 | XDG 配置路径感知 / XDG-aware config lookup | P2 | S | 部分 / Partial |
| B3 | 重复且不一致的 session 推导代码 / Duplicate session derivation | P2 | S | 未清理 / Open |
| B4 | 未使用依赖清理 / Unused dependency cleanup | P2 | S | 未清理 / Open |
| C1 | daemon/client 单元测试覆盖 / Unit test coverage | P1 | L | 缺失 / Missing |
| C2 | CI 增加 test / clippy / CI test & lint | P2 | S | 缺失 / Missing |
| D1–D5 | 明确不做的决策 / Deliberate non-goals | P3 | — | 记录在案 / Documented |

---

## A. 缺失功能 / Missing Features

### A1. 终端捕获 / Terminal Capture

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: `command_line_assistant/terminal/reader.py`（`TerminalRecorder` 通过 pty 监听，按提示符标记 `\x1b]` 切块，写 `~/.local/state/command-line-assistant/terminal.log` JSONL）、`terminal/parser.py`；`c shell --enable-capture` 安装 bashrc 片段并驱动捕获；捕获运行时交互模式被文件锁阻塞（`commands/chat.py:613-619`）
- **Rust 版现状 / Current state**: `crates/cla-client/src/cli.rs` 已声明 `shell --enable-capture` 参数；`terminal.rs` 已有 JSONL **解析器**（`-w/--with_output` 使用，按 1=最新 索引），但捕获器本身未实现 — `main.rs:546-548` 明确警告 *"Terminal capture is not yet implemented in the Rust version"*。因此 `-w` 读取的 `terminal.log` 实际不存在
- **建议实现要点 / Implementation notes**:
  1. 用 `nix`（已在依赖中，features 含 term/fs/process/poll）实现 PTY 捕获器，仿照上游的命令/输出切块逻辑
  2. `shell --enable-capture` 写入 shell rc 片段（参考上游安装方式）
  3. 交互模式与捕获的文件锁互斥（上游用 `fs2`，Rust 版已依赖 fs2）
  4. 补 `terminal.log` 写入端后，`-w` 立即可用，需配套测试

### A2. D-Bus 调用方授权校验 / Caller Authorization

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: `command_line_assistant/dbus/interfaces/authorization.py` — `DBusAuthorizationMixin` 在**每个接口方法**入口校验：`GetConnectionUnixUser(sender)` 必须匹配请求的 euid（`chat.py:47-84`）或推导出的内部 user id（`chat.py:86-126`），fail-closed；通过 `dbus/server.py` 的 `SpecialServerObjectHandler` 把 sender 注入 thread-local
- **Rust 版现状 / Current state**: `ChatInterface` / `HistoryInterface` / `UserInterface`（`crates/cla-daemon/src/{chat_interface.rs, history_interface.rs, user_interface.rs}`）对任何能连系统总线的调用方一视同仁，`AskQuestion` 可被任意用户冒用任意 euid 调用
- **建议实现要点 / Implementation notes**:
  1. zbus 4 可在方法签名外通过 `zbus::Connection` 的 `GetConnectionUnixUser` 或 `fdo::DBusProxy` 取 sender；利用 zbus 的 `#[zbus::interface]` 内 `&zbus::ObjectServer`/header 机制获取调用方身份
  2. 在 daemon 侧写一个 `AuthorizationMixin`-like 的 trait/helper，AskQuestion、WriteHistory、GetUserId 等入口统一调用
  3. 校验失败返回 `fdo::Error::AccessDenied`，客户端给出可读提示

### A3. D-Bus 自动激活 / D-Bus Activation

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: `data/release/dbus/com.redhat.lightspeed.{chat,history,user}.service`（`[D-BUS Service]`，`SystemdService=clad.service`，`Exec=/bin/false`）— 任意用户请求总线名时 systemd 自动拉起 clad
- **Rust 版现状 / Current state**: `config/clad.service` 虽是 `Type=dbus` + `BusName=com.redhat.lightspeed.chat`，但没有安装 `/usr/share/dbus-1/system-services/` 下的激活文件；daemon 未运行时客户端报 `Failed to get user ID`（systemd 无法自动拉起）
- **建议实现要点 / Implementation notes**:
  1. 新增三个 `[D-BUS Service]` 文件（chat/history/user → 指向 `clad.service`），`scripts/install.sh` / `uninstall.sh` 同步安装/卸载
  2. 注意 systemd unit 需允许 D-Bus 激活路径（保留 `Type=dbus`，`Restart=on-failure` 已具备）
  3. 验证：daemon 停止状态下直接 `c "question"` 应自动拉起

### A4. 审计日志 / Audit Logging

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: 配置 `[logging.audit]`（`config/schemas/logging.py`）→ journald 结构化事件
- **Rust 版现状 / Current state**: `config/config.toml` 模板有 `[logging.audit]` 注释示例，但 `cla-common/src/config.rs` 无对应字段、daemon 无审计事件；现有 `tracing` + `RUST_LOG` 仅普通日志
- **建议实现要点 / Implementation notes**: config.rs 增加 `logging.audit.enabled` 字段；daemon 在 AskQuestion/WriteHistory 等关键入口发结构化 tracing 事件（`tracing::event!(target: "audit", ...)`），由 journald 采集

### A5. SELinux 策略 / SELinux Policy

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: `data/release/selinux/{clad.te, clad.if, clad.fc, Makefile}` — 为 `clad` 定制策略模块
- **Rust 版现状 / Current state**: 无任何 SELinux 交付物；RHEL/Fedora 上安装后依赖默认策略
- **建议实现要点 / Implementation notes**: 以 `command-line-assistant` 策略为模板，改写为 `cli_assistant` 域（路径 `/usr/local/bin/clad`、`/var/lib/cli-assistant`、D-Bus 名称），配套 CI 或文档说明编译安装步骤

### A6. man pages 与 RPM 打包 / man Pages & RPM Packaging

- **状态 / Status**: 未实现 / Not implemented
- **上游参考 / Upstream reference**: `data/release/man/{c.1, clad.8}`；`packaging/command-line-assistant.spec` + `.packit.yaml` / `.gitlab-ci.yml` 下游打包
- **Rust 版现状 / Current state**: 仅有 `scripts/install.sh`（拷贝二进制 + D-Bus policy + systemd unit + 配置）；无 man pages、无 RPM spec
- **建议实现要点 / Implementation notes**: 用 clap 的 `generate_markdown`/`man` 能力生成 `c.1`/`clad.8`；spec 文件可基于上游改造（二进制静态，依赖几乎为零）；`cargo build --release` 产物 tar 打包已由 `.github/workflows/release.yml` 完成

### A7. XDG 配置路径感知 / XDG-aware Config Lookup

- **状态 / Status**: 部分 / Partial
- **上游参考 / Upstream reference**: `config/__init__.py:26-29, 56-82` + `utils/environment.py:70-111` — 支持 `XDG_CONFIG_DIRS`（如 `/etc/xdg/command-line-assistant/config.toml`）多路径查找
- **Rust 版现状 / Current state**: `Config::load()`（`cla-common/src/config.rs:236-251`）仅查 `/etc/cli-assistant/config.toml` → `$XDG_CONFIG_HOME/cli-assistant/config.toml` → 默认值；不读 `XDG_CONFIG_DIRS` 系统级多路径
- **建议实现要点 / Implementation notes**: 在 load 链中插入 `$XDG_CONFIG_DIRS/cli-assistant/config.toml` 各路径（最后一个找到的生效，与上游一致）；保持 `/etc/cli-assistant/config.toml` 作为最高优先级以便现有部署不受影响

---

## B. 回归缺陷 / Regressions (upstream correct → Rust broken)

### B1. 历史记录存错：question 被存成 response / History Stores the Response as Question

- **状态 / Status**: 未修复 / Open — **P0**
- **上游参考 / Upstream reference**: `commands/chat.py:379-412` — `WriteHistory(chat_id, user_id, message, response)` 传入真实问题
- **Rust 版现状 / Current state**: `crates/cla-client/src/main.rs:330-333`（单发）与 `main.rs:384-386`（交互模式）均为
  ```rust
  dbus.write_history(&chat_id, &user_id, &response.message, &response.message)
  ```
  **用户的问题文本从未入库** — `interactions` 表每行是回答内容的两份拷贝；`history --filter` 按问题搜索将永远无法命中
- **修复 / Fix**: 将第一个参数改为实际发送的 `question.message`（单发为 `question.message`，交互模式为当前迭代的消息）；补一个针对该行为的回归测试

### B2. `history.enabled` 配置未生效 / `history.enabled` Not Honored

- **状态 / Status**: 未修复 / Open — **P0**
- **上游参考 / Upstream reference**: 上游 daemon 检查 `history.enabled`，禁用时抛出 `HistoryNotEnabledError`，客户端 `_submit_question` 将其吞掉（不写历史、不报错）
- **Rust 版现状 / Current state**: `config.rs:136-145` 定义了 `history.enabled`，但 daemon 的 `LocalHistory::write`（`history/local.rs:99-117`）**无条件写入**，配置形同虚设
- **修复 / Fix**: `HistoryInterface::write_history`（`history_interface.rs:130-142`）入口检查 `config.history.enabled`；为保持与上游一致的体验，禁用时返回成功（或专用错误由客户端静默忽略），确保 `history` 命令的读取路径同样遵循该配置

### B3. 重复且不一致的 session 推导代码 / Duplicate Session Derivation

- **状态 / Status**: 未清理 / Open — **P2**
- **上游参考 / Upstream reference**: `daemon/session.py:51-64` — `uuid5(uuid(machine-id), euid)`
- **Rust 版现状 / Current state**: daemon 生效实现 `user_interface.rs:17-38` 与上游一致；但 `cla-common/src/session.rs:55-58` 存在**另一套**推导（`uuid5(NAMESPACE_DNS, "machine_id:euid")`，machine_id 按 UUID 解析）并经 `lib.rs:32` re-export，**未被任何代码使用**，两套逻辑不一致会误导后续开发
- **修复 / Fix**: 删除 `cla-common/src/session.rs` 中未使用的实现（或改为复用 `user_interface.rs` 的算法并消除重复）

### B4. 未使用依赖清理 / Unused Dependency Cleanup

- **状态 / Status**: 未清理 / Open — **P2**
- **上游参考 / Upstream reference**: 上游使用 `python-markdown` 扩展；Rust 版手写渲染器是其刻意替代
- **Rust 版现状 / Current state**: `cla-client/Cargo.toml` 声明 `termimad`（:21）、`crossterm`、`pulldown-cmark`，但代码中均未使用（grep 仅出现在清单行）— 渲染走自定义 `rendering/markdown.rs`
- **修复 / Fix**: 从 `Cargo.toml` 移除这三个依赖（若未来要接 `-w` 终端捕获的 ANSI 剥离可再评估），减小二进制体积与编译时间

---

## C. 测试与 CI / Testing & CI

### C1. daemon / client 单元测试覆盖 / Unit Test Coverage

- **状态 / Status**: 缺失 / Missing — **P1**
- **上游参考 / Upstream reference**: `tests/` 43 个 pytest 文件，覆盖命令解析、配置、DB、HTTP（`responses` mock）、D-Bus 接口、渲染、终端解析等；`tox.ini` + 覆盖率
- **Rust 版现状 / Current state**: `#[cfg(test)]` 仅存在于 `cla-common`（config/session/errors/files/environment/constants/system）与 `cla-dbus`（exceptions）；`cla-daemon` 的 HTTP 请求构造、`cla-client` 的参数解析/输入合成/渲染**零测试**
- **建议 / Recommendation**:
  1. `query.rs` 用 `httpmock` 或 mock reqwest 测 URL 构造、重试/退避、4xx/5xx/传输错误分支
  2. `main.rs` 的 `add_default_command`、`gather_input` 优先级、`MAX_QUESTION_SIZE` 截断、交互循环收敛为可测纯函数
  3. `rendering/markdown.rs` 各语法块（代码框/表格/列表/行内样式）快照测试
  4. 修复 B1/B2 时同步补回归测试

### C2. CI 增加 test / clippy / CI Test & Lint

- **状态 / Status**: 缺失 / Missing — **P2**
- **上游参考 / Upstream reference**: pre-commit 自动更新、`make unit-test`（uv + pytest）、tox 矩阵
- **Rust 版现状 / Current state**: `.github/workflows/release.yml` 仅做 tag 触发的交叉编译与打包；无 PR 检查
- **建议 / Recommendation**: 新增 `ci.yml`：`cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`（x86_64），PR/主分支触发；release workflow 保留

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
