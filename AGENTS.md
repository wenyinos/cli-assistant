# AGENTS.md

Workspace guidance for `cli-assistant` — a Rust rewrite of RHEL Lightspeed's command-line-assistant: a fast CLI assistant for Linux system administration backed by any OpenAI-compatible API.

## Layout

Cargo workspace (`resolver = "2"`, edition 2021) with 4 crates:

| Crate | Role |
|---|---|
| `crates/cla-common` | Shared: config, errors, session, file/env utils, system info (most unit tests live here) |
| `crates/cla-dbus` | D-Bus interface names, object paths, exception conversions (constants only) |
| `crates/cla-client` | CLI binary `c` (clap 4 derive, markdown→ANSI renderer) |
| `crates/cla-daemon` | Daemon binary `clad`: D-Bus server, HTTP client, SQLite history |

## Architecture

```
c (client) ──D-Bus (system bus)──▶ clad (daemon) ──HTTP──▶ LLM API
                                     │
                                     └── SQLite (history)
```

- Client and daemon talk only over D-Bus on the **system bus**. No direct IPC, no sockets.
- `clad` runs as root/systemd service; `c` runs as any user. Daemon requires root or D-Bus policy (`config/com.cli-assistant.conf`).
- D-Bus names use the `com.redhat.lightspeed.*` namespace (inherited from the Python original) — all identifiers are defined as constants in `cla-dbus/src/constants.rs`; keep them in sync with the client and daemon.

## Build / Test / Lint

```bash
cargo check                       # type check (fastest)
cargo build --release             # binaries: target/release/c and target/release/clad
cargo build -p cla-common         # single crate
cargo test                        # unit tests (cla-common, cla-dbus); daemon/client are runtime-integration only
cargo test -p cla-common config::tests::default_config_roundtrips
```

No clippy/rustfmt config in-repo; CI (`.github/workflows/release.yml`) builds `x86_64` and `aarch64` release binaries on tag pushes. `Cargo.lock` is **gitignored** (unusual for a binary project — intended).

## Conventions & gotchas

- **Endpoint URLs (v0.6.5+):** `backend.endpoint` in config includes the API version path (default `https://api.openai.com/v1`). `BackendConfig::chat_completions_url()` (cla-common/src/config.rs:100) appends only `/chat/completions` and trims a trailing `/`. Never append `/v1` again — mirrors with custom version paths (e.g. `/v2`) are supported.
- **Config:** `/etc/cli-assistant/config.toml`. `CL_API_KEY` env var overrides `backend.api_key`. The daemon must be restarted (`sudo systemctl restart clad`) after config changes. Default DB path is `/var/lib/cli-assistant/cla.db` (systemd-compatible), even though the README example shows `~/.local/share/...`.
- **Logging:** `tracing` + `tracing-subscriber`; control via `RUST_LOG` (e.g. `RUST_LOG=cla_daemon::http=debug`). `NO_COLOR` disables ANSI colors.
- **Version bumps:** `workspace.package.version` (currently 0.6.5). Version-stamped releases also touch README.md, README.zh-CN.md, docs/BUILD.md, and config/config.toml in sync — check commit history for the pattern.
- **Docs are bilingual:** README.md (EN), README.zh-CN.md (CN), docs/BUILD.md (bilingual inline). When editing one, update the mirror.
- **Testing the daemon** requires the D-Bus system bus and root: `sudo RUST_LOG=debug ./target/debug/clad`, then `./target/debug/c "question"` in another shell.

## Before changing sensitive areas

- Endpoint/URL building → `crates/cla-common/src/config.rs` (tests included there).
- D-Bus names/paths → `crates/cla-dbus/src/constants.rs`, and the policy file `config/com.cli-assistant.conf`.
- Install/uninstall/systemd → `scripts/install.sh`, `scripts/uninstall.sh`, `config/cli-assistant.service`.
- Read `docs/BUILD.md` and the matching README (language of your choice) before touching build or runtime behavior.
