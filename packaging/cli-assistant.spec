%global binary_name c
%global daemon_binary_name clad
%global selinuxtype targeted
%global modulename cli_assistant

Name:           cli-assistant
Version:        0.9.1
Release:        1%{?dist}
Summary:        Command Line Assistant client and daemon

License:        MIT
URL:            https://github.com/wenyinos/cli-assistant
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  systemd
BuildRequires:  selinux-policy-devel

Requires:       systemd
Requires:       %{name}-selinux

%description
cli-assistant is a fast CLI assistant for Linux system administration backed
by any OpenAI-compatible API. The client communicates with the clad daemon
over D-Bus, and the daemon stores conversation history in SQLite.

%package selinux
Summary:    cli-assistant SELinux policy
Requires(post): selinux-policy-%{selinuxtype}

%description selinux
This package installs the SELinux policy module for the clad daemon.

%prep
%autosetup -n cli-assistant-%{version}

%build
cargo build --release

pushd data/release/selinux
%{__make} %{modulename}.pp.bz2
popd

%install
%{__install} -d %{buildroot}/usr/local/bin
%{__install} -d %{buildroot}/etc/cli-assistant
%{__install} -d %{buildroot}/etc/systemd/system
%{__install} -d %{buildroot}/etc/dbus-1/system.d
%{__install} -d %{buildroot}/usr/share/dbus-1/system-services
%{__install} -d %{buildroot}/usr/local/share/man/man1
%{__install} -d %{buildroot}/usr/local/share/man/man8
%{__install} -d %{buildroot}/var/lib/cli-assistant
%{__install} -d %{buildroot}/usr/share/selinux/packages/%{selinuxtype}

%{__install} -m 0755 target/release/%{binary_name} %{buildroot}/usr/local/bin/%{binary_name}
%{__install} -m 0755 target/release/%{daemon_binary_name} %{buildroot}/usr/local/bin/%{daemon_binary_name}

%{__install} -m 0644 config/clad.service %{buildroot}/etc/systemd/system/clad.service

%{__install} -m 0644 config/com.cli-assistant.conf \
    %{buildroot}/etc/dbus-1/system.d/com.cli-assistant.conf
%{__install} -m 0644 config/com.redhat.lightspeed.chat.service \
    %{buildroot}/usr/share/dbus-1/system-services/com.redhat.lightspeed.chat.service
%{__install} -m 0644 config/com.redhat.lightspeed.history.service \
    %{buildroot}/usr/share/dbus-1/system-services/com.redhat.lightspeed.history.service
%{__install} -m 0644 config/com.redhat.lightspeed.user.service \
    %{buildroot}/usr/share/dbus-1/system-services/com.redhat.lightspeed.user.service

%{__install} -m 0644 data/release/man/%{binary_name}.1 %{buildroot}/usr/local/share/man/man1/
%{__install} -m 0644 data/release/man/%{daemon_binary_name}.8 %{buildroot}/usr/local/share/man/man8/

%{__install} -m 0644 data/release/selinux/%{modulename}.pp.bz2 \
    %{buildroot}/usr/share/selinux/packages/%{selinuxtype}/%{modulename}.pp.bz2

%preun
%systemd_preun clad.service

%post
%systemd_post clad.service
echo "cli-assistant installed. Run 'sudo c setup' to configure the backend."

%postun
%systemd_postun_with_restart clad.service

%pre selinux
%selinux_relabel_pre -s %{selinuxtype}

%post selinux
%selinux_modules_install -s %{selinuxtype} %{_datadir}/selinux/packages/%{selinuxtype}/%{modulename}.pp.bz2

%postun selinux
if [ $1 -eq 0 ]; then
    %selinux_modules_uninstall -s %{selinuxtype} %{modulename}
fi

%posttrans selinux
%selinux_relabel_post -s %{selinuxtype}

%files
%license LICENSE
%doc README.md
/usr/local/bin/%{binary_name}
/usr/local/bin/%{daemon_binary_name}
/etc/systemd/system/clad.service
/etc/dbus-1/system.d/com.cli-assistant.conf
/usr/share/dbus-1/system-services/com.redhat.lightspeed.chat.service
/usr/share/dbus-1/system-services/com.redhat.lightspeed.history.service
/usr/share/dbus-1/system-services/com.redhat.lightspeed.user.service
%dir %attr(0755, root, root) /etc/cli-assistant
/usr/local/share/man/man1/%{binary_name}.1*
/usr/local/share/man/man8/%{daemon_binary_name}.8*
%dir %attr(0700, root, root) /var/lib/cli-assistant

%files selinux
%attr(0600, root, root) %{_datadir}/selinux/packages/%{selinuxtype}/%{modulename}.pp.bz2

%changelog
* Wed Sep 23 2026 cli-assistant contributors - 0.9.1-1
- Interactive shell integration supports bash and zsh, written into the shell
  rc file with a removable marker block; `c feedback` points at the project
  issue tracker.
- The TUI conversation wraps long messages and renders markdown; `c --help`
  gained an EXAMPLES section and word-wrapped output.

* Wed Sep 23 2026 cli-assistant contributors - 0.9.0-1
- Multi-turn conversation context for the interactive/TUI pages, with automatic
  compaction of long histories into a model-written summary.
- New `c setup` first-run configuration wizard; packages no longer ship a
  default /etc/cli-assistant/config.toml.
- New `context_length` backend setting; DeepSeek defaults (deepseek-v4-flash,
  expanded system prompt).
- deb and pacman packaging added; all packages build for x86_64 and aarch64.

* Mon Aug 03 2026 cli-assistant contributors - 0.8.2-1
- Harden install/uninstall scripts: systemd check, RPM ownership detection,
  overridable paths.

* Mon Aug 03 2026 cli-assistant contributors - 0.8.1-1
- Load config exclusively from /etc/cli-assistant/config.toml (no XDG lookup).
- Align RPM install paths with the tarball installer.

* Mon Aug 03 2026 cli-assistant contributors - 0.8.0-1
- Initial RPM packaging for the Rust rewrite.
