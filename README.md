[English](README.md) | [中文](README.zh-CN.md)

# cli-assistant

A fast, lightweight CLI assistant for Linux system administration — powered by any OpenAI-compatible API.

> **Inspired by** [command-line-assistant](https://github.com/rhel-lightspeed/command-line-assistant) by RHEL Lightspeed. This is a Rust rewrite that improves portability, performance, and flexibility.

## Features

- **Ask questions in natural language** from your terminal
- **OpenAI-compatible** — works with OpenAI, Azure OpenAI, local LLMs, or any `/v1/chat/completions` endpoint
- **Chat sessions & history** — persistent SQLite-backed conversation history
- **Interactive mode** — continuous conversation with context
- **Markdown rendering** — colored terminal output with code blocks, tables, and headers
- **Configurable language** — force replies in your preferred language
- **D-Bus daemon architecture** — client/daemon separation for system-level integration

## Installation

### From Release (Recommended)

Download the latest tarball from [Releases](../../releases), then run the install script:

```bash
# Download (replace VERSION with actual version, e.g. v0.6.0)
curl -LO https://github.com/rhel-lightspeed/cli-assistant/releases/download/VERSION/cli-assistant-x86_64-linux-gnu.tar.gz

# Extract
tar xzf cli-assistant-x86_64-linux-gnu.tar.gz
cd cli-assistant

# Install (requires root)
sudo ./install.sh
```

The install script will:
- Copy binaries (`c`, `clad`) to `/usr/local/bin`
- Install D-Bus policy to `/etc/dbus-1/system.d/`
- Register `clad` as a systemd service
- Write default config to `/etc/cli-assistant/config.toml`

```bash
# Edit config — set your API key and endpoint
sudo vim /etc/cli-assistant/config.toml

# Test
c "How do I check disk space?"
```

### From Source

```bash
# Build
cargo build --release

# Configure
sudo mkdir -p /etc/cli-assistant
sudo cp config/config.toml /etc/cli-assistant/config.toml
# Edit /etc/cli-assistant/config.toml — set your API key and endpoint

# Install D-Bus policy
sudo cp config/com.cli-assistant.conf /etc/dbus-1/system.d/

# Run
sudo ./target/release/clad &          # start daemon
./target/release/c "How do I check disk space?"  # ask a question
```

For detailed build, test, and run instructions, see **[docs/BUILD.md](docs/BUILD.md)**.

### Uninstall

```bash
sudo ./scripts/uninstall.sh
```

## Configuration

Config file: `/etc/cli-assistant/config.toml`

```toml
[backend]
endpoint  = "https://api.openai.com"   # any OpenAI-compatible endpoint
model     = "gpt-4"
api_key   = "sk-..."
prompt    = "You are a helpful assistant for Linux system administration."
language  = "zh-CN"                     # reply language (empty = auto)
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

The API key can also be set via the `CL_API_KEY` environment variable (takes precedence over config).

## Usage

```bash
c "question"                    # ask a question (default: chat)
c chat "question"               # same as above
c chat --interactive            # interactive conversation mode
c chat -a /path/to/file "explain this"  # attach a file
c history --all                 # view all history
c history --filter "keyword"    # search history
c history --clear               # clear history for a chat
c feedback                      # show feedback info
c shell --enable-interactive    # enable Ctrl+G shortcut
```

## Improvements Over the Original

| Area | Python Original | This Project (Rust) |
|---|---|---|
| **Language** | Python 3.9+ | Rust (edition 2021) |
| **Runtime** | CPython + pip dependencies | Single static binary, no runtime deps |
| **API Backend** | RHEL Lightspeed only | Any OpenAI-compatible endpoint |
| **LLM Config** | Hardcoded backend | Configurable model, key, prompt, temperature, max_tokens, language |
| **Database** | SQLAlchemy (SQLite/MySQL/PostgreSQL) | sqlx + SQLite only (simpler, lighter) |
| **IPC** | dasbus (Python D-Bus) | zbus 4.x (native async Rust D-Bus) |
| **HTTP** | requests + urllib3 | reqwest + rustls (async, no OpenSSL) |
| **CLI** | argparse with decorator pattern | clap 4 derive (type-safe, auto-completions) |
| **Rendering** | python-markdown → ANSI | Custom markdown→ANSI renderer |
| **Dependencies** | ~10 Python packages | Pure Rust crates, vendored via Cargo |
| **Startup** | ~200ms (Python import) | ~5ms (native binary) |
| **Docker/CI** | Included but complex | Not needed — just `cargo build` |

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
| `cla-client` | CLI parser, renderer, D-Bus client |
| `cla-daemon` | D-Bus server, HTTP client, SQLite storage, history |

## License

MIT
