[English](README.md) | [中文](README.zh-CN.md)

# cli-assistant

A fast, lightweight CLI assistant for Linux system administration — powered by any OpenAI-compatible API.

> **Inspired by** [command-line-assistant](https://github.com/rhel-lightspeed/command-line-assistant) by RHEL Lightspeed. This is a Rust rewrite that improves portability, performance, and flexibility.

## Features

- **Ask questions in natural language** from your terminal
- **OpenAI-compatible** — works with OpenAI, Azure OpenAI, local LLMs, or any OpenAI-compatible endpoint
- **Setup wizard** — `sudo c setup` configures the endpoint, API key, model (pickable from the endpoint's `/models` list) and reply language
- **Chat sessions & history** — persistent SQLite-backed conversation history
- **Interactive mode** — line-based and full-screen TUI conversations that carry recent turns as context and compact older history into a summary automatically
- **Markdown rendering** — colored terminal output with code blocks, tables, and headers
- **Configurable language** — force replies in your preferred language
- **D-Bus daemon architecture** — client/daemon separation with system activation and caller authorization
- **Terminal capture** — include recent shell output as question context with `c shell --enable-capture`
- **Audit logging** — structured audit events through journald-style tracing

## Installation

Prebuilt packages for **x86_64** and **aarch64** are published on the
[Releases](../../releases) page:

| Distribution | Package | Install |
|---|---|---|
| Fedora / RHEL | `.rpm` | `sudo dnf install ./cli-assistant-*.rpm` |
| Debian / Ubuntu | `.deb` | `sudo apt install ./cli-assistant_*.deb` |
| Arch Linux | `.pkg.tar.zst` | `sudo pacman -U ./cli-assistant-*.pkg.tar.zst` |

RPM and DEB ship x86_64/amd64 and aarch64/arm64; the pacman package is built
for x86_64 (Arch Linux ARM builds the aarch64 package locally from
`packaging/PKGBUILD`).

Building a package from this repository instead:

```bash
# RPM (needs the source tarball in ~/rpmbuild/SOURCES; CI creates it on tags)
rpmbuild -ba packaging/cli-assistant.spec

# Debian / Ubuntu (native package, builds in place)
dpkg-buildpackage -us -uc -b

# Arch Linux — and Arch Linux ARM, which builds the aarch64 package locally
cd packaging && makepkg -s
```

The packages install the binaries (`c`, `clad`), D-Bus policy and activation
files, the systemd unit, and the man pages, and prepare `/etc/cli-assistant/`.
The config file itself is created by the setup wizard:

```bash
# Configure the backend (first run) — prompts for endpoint, API key, model
# (selectable from the endpoint's /models list) and reply language, writes
# /etc/cli-assistant/config.toml and restarts clad
sudo c setup

# Test
c "How do I check disk space?"
```

### From Source

```bash
# Build
cargo build --release

# Configure — writes /etc/cli-assistant/config.toml and restarts clad
sudo ./target/release/c setup

# Install D-Bus policy
sudo cp config/com.cli-assistant.conf /etc/dbus-1/system.d/

# Run
sudo ./target/release/clad &          # start daemon
./target/release/c "How do I check disk space?"  # ask a question
```

For detailed build, test, and run guidance, see **[AGENTS.md](AGENTS.md)**.

### Uninstall

```bash
sudo dnf remove cli-assistant cli-assistant-selinux   # Fedora / RHEL
sudo apt remove cli-assistant                          # Debian / Ubuntu
sudo pacman -R cli-assistant                           # Arch Linux
```

Configuration (`/etc/cli-assistant/`) and data (`/var/lib/cli-assistant/`) are
preserved; remove them manually if you want a clean slate.

### Service Management

```bash
sudo systemctl status clad     # Check service status
sudo systemctl restart clad    # Restart (required after manual config edits)
sudo systemctl stop clad       # Stop the daemon
sudo systemctl start clad      # Start the daemon
sudo systemctl enable clad     # Enable on boot (done by the package)
sudo systemctl disable clad    # Disable on boot
journalctl -u clad -f          # View live logs
```

## Configuration

Config file: `/etc/cli-assistant/config.toml` — created by the `sudo c setup` wizard.
Restart the daemon after manual edits (`sudo systemctl restart clad`).

```toml
[backend]
endpoint  = "https://api.deepseek.com/v1"   # any OpenAI-compatible endpoint
model     = "deepseek-v4-flash"
api_key   = "sk-..."
prompt    = "You are a command-line assistant for Linux system administration. Answer concisely and accurately, and prefer standard, widely available tools. Keep commands copy-pasteable; before any destructive or irreversible step, explain what it does and call out the risk. If a request is ambiguous, state your assumption briefly and answer the most likely intent."
language  = "zh-CN"                     # reply language (empty = auto)
max_tokens     = 32768
context_length = 256000                 # model context window (drives auto-compaction)
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

The API key can also be set via the `CL_API_KEY` environment variable (takes precedence over config).
The daemon loads its configuration exclusively from `/etc/cli-assistant/config.toml` on all
distributions.

## Usage

```bash
sudo c setup                    # first-run wizard: endpoint, API key, model, reply language
c "question"                    # ask a question (default: chat)
c chat "question"               # same as above
c chat --interactive            # interactive conversation mode
c chat --tui                    # full-screen TUI conversation mode
c chat -a /path/to/file "explain this"  # attach a file
c history --all                 # view all history
c history --filter "keyword"    # search history
c history --clear               # clear history for a chat
c feedback                      # where to report bugs and request features
c shell --enable-interactive    # enable the Ctrl+G shortcut (bash and zsh)
```

## Improvements Over the Original

| Area | Python Original | This Project (Rust) |
|---|---|---|
| **Language** | Python 3.9+ | Rust (edition 2021) |
| **Runtime** | CPython + pip dependencies | Single static binary, no runtime deps |
| **API Backend** | RHEL Lightspeed only | Any OpenAI-compatible endpoint |
| **LLM Config** | Hardcoded backend | Configurable model, key, prompt, temperature, max_tokens, context_length, language |
| **Database** | SQLAlchemy (SQLite/MySQL/PostgreSQL) | sqlx + SQLite only (simpler, lighter) |
| **IPC** | dasbus (Python D-Bus) | zbus 4.x (native async Rust D-Bus) |
| **HTTP** | requests + urllib3 | reqwest + rustls (async, no OpenSSL) |
| **CLI** | argparse with decorator pattern | clap 4 derive (type-safe, auto-completions) |
| **Rendering** | python-markdown → ANSI | Custom markdown→ANSI renderer |
| **Dependencies** | ~10 Python packages | Pure Rust crates, vendored via Cargo |
| **Startup** | ~200ms (Python import) | ~5ms (native binary) |
| **Docker/CI** | Included but complex | GitHub Actions: fmt/clippy/test, plus rpm/deb/pacman builds for x86_64 and aarch64 |

## Architecture

```
c (client)  ──D-Bus──▶  clad (daemon)  ──HTTP──▶  LLM API
                           │
                           └── SQLite (history)
```

| Crate | Purpose |
|---|---|
| `cla-common` | Config, errors, session, file utils, system info |
| `cla-dbus` | D-Bus interface definitions & data structures |
| `cla-client` | CLI parser, renderer, setup wizard, D-Bus client |
| `cla-daemon` | D-Bus server, HTTP client, SQLite storage, history |

## License

MIT
