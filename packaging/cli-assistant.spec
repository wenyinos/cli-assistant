%global binary_name c
%global daemon_binary_name clad
%global selinuxtype targeted
%global modulename cli_assistant

Name:           cli-assistant
Version:        0.8.0
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
%{__install} -d %{buildroot}%{_bindir}
%{__install} -d %{buildroot}%{_unitdir}
%{__install} -d %{buildroot}%{_datadir}/dbus-1/system.d
%{__install} -d %{buildroot}%{_datadir}/dbus-1/system-services
%{__install} -d %{buildroot}%{_sysconfdir}/xdg/command-line-assistant
%{__install} -d %{buildroot}%{_sharedstatedir}/cli-assistant
%{__install} -d %{buildroot}%{_mandir}/man1
%{__install} -d %{buildroot}%{_mandir}/man8
%{__install} -d %{buildroot}%{_datadir}/selinux/packages/%{selinuxtype}

%{__install} -m 0755 target/release/%{binary_name} %{buildroot}%{_bindir}/%{binary_name}
%{__install} -m 0755 target/release/%{daemon_binary_name} %{buildroot}%{_bindir}/%{daemon_binary_name}

sed "s|/usr/local/bin/clad|%{_bindir}/clad|" \
    config/clad.service > %{buildroot}%{_unitdir}/clad.service

%{__install} -m 0644 config/com.cli-assistant.conf \
    %{buildroot}%{_datadir}/dbus-1/system.d/com.cli-assistant.conf
%{__install} -m 0644 config/com.redhat.lightspeed.chat.service \
    %{buildroot}%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.chat.service
%{__install} -m 0644 config/com.redhat.lightspeed.history.service \
    %{buildroot}%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.history.service
%{__install} -m 0644 config/com.redhat.lightspeed.user.service \
    %{buildroot}%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.user.service

%{__install} -m 0600 config/config.toml \
    %{buildroot}%{_sysconfdir}/xdg/command-line-assistant/config.toml

%{__install} -m 0644 data/release/man/%{binary_name}.1 %{buildroot}%{_mandir}/man1/
%{__install} -m 0644 data/release/man/%{daemon_binary_name}.8 %{buildroot}%{_mandir}/man8/

%{__install} -m 0644 data/release/selinux/%{modulename}.pp.bz2 \
    %{buildroot}%{_datadir}/selinux/packages/%{selinuxtype}/%{modulename}.pp.bz2

%preun
%systemd_preun clad.service

%post
%systemd_post clad.service

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
%{_bindir}/%{binary_name}
%{_bindir}/%{daemon_binary_name}
%{_unitdir}/clad.service
%{_datadir}/dbus-1/system.d/com.cli-assistant.conf
%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.chat.service
%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.history.service
%{_datadir}/dbus-1/system-services/com.redhat.lightspeed.user.service
%config(noreplace) %attr(0600, root, root) %{_sysconfdir}/xdg/command-line-assistant/config.toml
%{_mandir}/man1/%{binary_name}.1.gz
%{_mandir}/man8/%{daemon_binary_name}.8.gz
%dir %attr(0700, root, root) %{_sharedstatedir}/cli-assistant

%files selinux
%attr(0600, root, root) %{_datadir}/selinux/packages/%{selinuxtype}/%{modulename}.pp.bz2

%changelog
* Mon Aug 03 2026 cli-assistant contributors - 0.8.0-1
- Initial RPM packaging for the Rust rewrite.
