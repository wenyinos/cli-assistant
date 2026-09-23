# AGENTS.md

Workspace guidance for `cli-assistant` — a Rust rewrite of RHEL Lightspeed's command-line-assistant: a fast CLI assistant for Linux system administration backed by any OpenAI-compatible API.

## Layout

Cargo workspace (`resolver = "2"`, edition 2021) with 4 crates:

| Crate | Role |
|---|---|
| `crates/cla-common` | Shared: config + template rendering, errors, session, file/env utils, system info (most unit tests live here) |
| `crates/cla-dbus` | D-Bus interface names, object paths, exception conversions (constants only) |
| `crates/cla-client` | CLI binary `c` (clap 4 derive, markdown→ANSI renderer, `c setup` wizard) |
| `crates/cla-daemon` | Daemon binary `clad`: D-Bus server, HTTP client, SQLite history |

Prerequisites: Linux with a D-Bus system bus, Rust 1.75+ (edition 2021), SQLite 3.x (the history database is created at runtime).

## Architecture

```
c (client) ──D-Bus (system bus)──▶ clad (daemon) ──HTTP──▶ LLM API
                                     │
                                     └── SQLite (history)
```

- Client and daemon talk only over D-Bus on the **system bus**. No direct IPC, no sockets. One deliberate exception: `c setup` talks to the backend's `/models` endpoint itself, because it must work before any usable config (and therefore any usable daemon) exists.
- `clad` runs as root/systemd service; `c` runs as any user. Daemon requires root or D-Bus policy (`config/com.cli-assistant.conf`).
- D-Bus names use the `com.redhat.lightspeed.*` namespace (inherited from the Python original) — all identifiers are defined as constants in `cla-dbus/src/constants.rs`; keep them in sync with the client and daemon.

## Build / Test / Lint

```bash
cargo check                       # type check (fastest)
cargo build                       # debug binaries: target/debug/{c,clad}
cargo build --release             # release binaries: target/release/{c,clad}
cargo build -p cla-common         # single crate
cargo test                        # unit tests across all crates
cargo test -p cla-common config::tests::default_config_roundtrips
```

CI: `.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` on pushes to main and PRs; `.github/workflows/release.yml` builds the x86_64 RPM on tag pushes. `Cargo.lock` is **gitignored** (unusual for a binary project — intended).

## Conventions & gotchas

- **Endpoint URLs (v0.6.5+):** `backend.endpoint` in config includes the API version path (default `https://api.deepseek.com/v1`). `BackendSchema::chat_completions_url()` (crates/cla-common/src/config.rs) appends only `/chat/completions` and trims a trailing `/`. Never append `/v1` again — mirrors with custom version paths (e.g. `/v2`) are supported. `models_url()` follows the same rule for the setup wizard.
- **Config:** `/etc/cli-assistant/config.toml` — created by the `sudo c setup` wizard (endpoint → API key → model, pickable from the endpoint's `/models` list → reply language). Re-running it edits in place and preserves every field it does not ask about. `CL_API_KEY` overrides `backend.api_key` and satisfies the configured-check on its own. The daemon must be restarted after manual edits (the wizard restarts it automatically). Default DB path is `/var/lib/cli-assistant/cla.db` — the daemon's systemd sandbox (`ProtectHome=read-only`) means user-home paths do not work.
- **Config template:** the wizard renders the repo template `config/config.toml`, embedded via `include_str!` in `cla-common/src/config.rs`; only `endpoint`/`model`/`api_key`/`language` inside `[backend]` are substituted, so all comments survive. Values are emitted as escaped TOML literals so arbitrary keys cannot break the file.
- **Missing config:** `c chat` exits with a pointer to `sudo c setup` when no config file exists and `CL_API_KEY` is unset (`setup.rs::check_configured`). History/shell/feedback are not gated.
- **Conversation context:** single-shot questions (`c "q"`) send only the system prompt plus the question. The interactive/TUI pages set `Question.context_chat` (chat name); the daemon then attaches that chat's unsummarized turns and, when they outgrow `backend.context_length`, folds the oldest turns into `histories.summary` with one extra model call (folded turns are flagged through `interactions.summarized`). `max_tokens` still bounds a single reply, not the conversation.
- **DB schema changes:** `DatabaseManager::migrate()` (cla-daemon/src/database/manager.rs) adds columns to databases created by older versions — there is no sqlx migration suite, so register new columns there and keep the check idempotent.
- **Logging:** `tracing` + `tracing-subscriber`; control via `RUST_LOG` (e.g. `RUST_LOG=cla_daemon::http=debug`, `RUST_LOG=trace` for maximum verbosity). `NO_COLOR` disables ANSI colors.
- **The setup wizard must stay D-Bus-free:** `crates/cla-client/src/setup.rs` is the only place the client uses HTTP (`reqwest`); it runs before a configured daemon exists, so it cannot go through D-Bus.
- **Version bumps:** `workspace.package.version` (currently 0.9.0). A release also touches `packaging/cli-assistant.spec` (Version + changelog), `debian/changelog`, `packaging/PKGBUILD` (pkgver), and the man pages' `.TH` line — grep for the old version string before cutting a tag.
- **Docs are bilingual:** README.md (EN) and README.zh-CN.md (CN). When editing one, update the mirror.
- **Testing the daemon** requires the D-Bus system bus and root: `sudo RUST_LOG=debug ./target/debug/clad`, then `./target/debug/c "question"` in another shell. Common failures: `Connection refused` → daemon not running; `Permission denied` → D-Bus policy, run with `sudo`; `No configuration found` / `No API key configured` → `sudo c setup`; `SQLite error` → check `database.path` permissions.

## Packaging

Distribution packages are the supported install path; there is no tarball/script installer. CI (`.github/workflows/release.yml`) builds all three on tag pushes: RPM and DEB for `x86_64`/`aarch64`, pacman for `x86_64` (Arch's official repositories and container images are x86_64-only).

- **RPM:** `packaging/cli-assistant.spec` — static binaries plus a SELinux subpackage. Needs a source tarball at `~/rpmbuild/SOURCES/v<version>.tar.gz` whose top-level directory is `cli-assistant-<version>/` (CI creates it with `git archive`). Installs under `/usr/local`, owns only the `/etc/cli-assistant` directory (not the config file), and `%post` points users at `sudo c setup`.
- **deb:** `debian/` — a native `3.0` package. `debian/rules` builds with plain cargo and installs to standard Debian paths (`/usr/bin`, `/usr/lib/systemd/system`, `/usr/share/dbus-1/{system.d,system-services}`, `/usr/share/man`); `dpkg-buildpackage -us -uc -b` builds it in place. `debian/cli-assistant.postinst` carries the `sudo c setup` hint.
- **pacman:** `packaging/PKGBUILD` plus `packaging/cli-assistant.install` (post-install hint). `arch=('x86_64' 'aarch64')`; Arch Linux ARM users build the aarch64 package locally with `makepkg`. The PKGBUILD sets `LIBSQLITE3_SYS_USE_PKG_CONFIG=1` so the package links the system SQLite — the bundled copy fails to resolve its symbols under Arch's toolchain (rust-lld plus makepkg's hardened CFLAGS/LDFLAGS), and sqlite is a base package there. Fedora and Debian builds keep the bundled (static) SQLite.
- **Install paths differ by package:** RPM keeps `/usr/local/bin` (upgrade compatibility with earlier releases), while deb and pacman use `/usr/bin`. The systemd unit exists in two variants for this reason — `config/clad.service` (in-repo/`/usr/local`) and `debian/cli-assistant.clad.service` (`/usr/bin`); the PKGBUILD rewrites the path with `sed` instead.
- **SELinux:** `make -C data/release/selinux cli_assistant.pp.bz2 && sudo semodule -i data/release/selinux/cli_assistant.pp`. Fedora-only — the deb/pacman packages carry no policy module, only the units' systemd hardening.

## Before changing sensitive areas

- Endpoint/URL building → `crates/cla-common/src/config.rs` (tests included there).
- Setup wizard + config rendering → `crates/cla-client/src/setup.rs` and `BackendSchema::render_into_template()` in `crates/cla-common/src/config.rs`.
- D-Bus names/paths → `crates/cla-dbus/src/constants.rs`, and the policy file `config/com.cli-assistant.conf`.
- Packaging/systemd → `packaging/cli-assistant.spec`, `config/clad.service`.
- Read the matching README (EN or CN) before touching user-facing build or runtime behavior.
