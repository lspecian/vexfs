Name:           vexfs
Version:        1.0.0
Release:        1%{?dist}
Summary:        Vector Extended File System - A filesystem optimized for vector embeddings

License:        Apache-2.0
URL:            https://github.com/vexfs/vexfs
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  openssl-devel
BuildRequires:  pkg-config
BuildRequires:  systemd-rpm-macros

Requires:       openssl-libs >= 3.0.0
Requires:       ca-certificates
Requires(pre):  shadow-utils
Requires(post): systemd
Requires(preun): systemd
Requires(postun): systemd

%description
VexFS is a high-performance filesystem specifically designed for storing and
querying vector embeddings. It provides ChromaDB-compatible APIs and supports
similarity search operations with advanced indexing capabilities.

Key features:
- ChromaDB-compatible REST API
- High-performance vector similarity search
- ANNS (Approximate Nearest Neighbor Search) support
- Production-ready with monitoring and logging
- Horizontal scaling capabilities

%prep
%setup -q

%build
# Build the VexFS server
cargo build --release --features server --bin vexfs_server

%install
# Create directories
install -d %{buildroot}%{_bindir}
install -d %{buildroot}%{_unitdir}
install -d %{buildroot}%{_sysconfdir}/vexfs
install -d %{buildroot}%{_sharedstatedir}/vexfs
install -d %{buildroot}%{_localstatedir}/log/vexfs
install -d %{buildroot}%{_tmpfilesdir}
install -d %{buildroot}%{_sysconfdir}/logrotate.d

# Install binary
install -m 755 target/release/vexfs_server %{buildroot}%{_bindir}/vexfs_server

# Install systemd service file
cat > %{buildroot}%{_unitdir}/vexfs.service << 'EOF'
[Unit]
Description=VexFS - Vector Extended File System Server
Documentation=https://github.com/vexfs/vexfs
After=network.target
Wants=network.target

[Service]
Type=exec
User=vexfs
Group=vexfs
ExecStart=%{_bindir}/vexfs_server
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
TimeoutStartSec=30
TimeoutStopSec=30

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=%{_sharedstatedir}/vexfs %{_localstatedir}/log/vexfs /run/vexfs
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictSUIDSGID=true
LockPersonality=true
MemoryDenyWriteExecute=true
RestrictNamespaces=true
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
EnvironmentFile=-%{_sysconfdir}/vexfs/vexfs.conf

# Working directory
WorkingDirectory=%{_sharedstatedir}/vexfs

# Standard streams
StandardOutput=journal
StandardError=journal
SyslogIdentifier=vexfs

[Install]
WantedBy=multi-user.target
EOF

# Install configuration file
cat > %{buildroot}%{_sysconfdir}/vexfs/vexfs.conf << 'EOF'
# VexFS Configuration File
# See documentation at: https://github.com/vexfs/vexfs

# Server configuration
PORT=8000
BIND_ADDRESS=127.0.0.1

# Data directory
VEXFS_DATA_DIR=%{_sharedstatedir}/vexfs

# Logging
VEXFS_LOG_LEVEL=info
RUST_LOG=info

# Performance tuning
VEXFS_MAX_CONNECTIONS=1000
VEXFS_REQUEST_TIMEOUT=30s

# Rate limiting
VEXFS_RATE_LIMIT_REQUESTS=100
VEXFS_RATE_LIMIT_WINDOW=60s

# Security
VEXFS_TLS_ENABLED=false
VEXFS_CORS_ENABLED=true

# Monitoring
VEXFS_METRICS_ENABLED=true
VEXFS_HEALTH_CHECK_ENABLED=true
EOF

# Install tmpfiles configuration
cat > %{buildroot}%{_tmpfilesdir}/vexfs.conf << 'EOF'
d /run/vexfs 0755 vexfs vexfs -
EOF

# Install logrotate configuration
cat > %{buildroot}%{_sysconfdir}/logrotate.d/vexfs << 'EOF'
%{_localstatedir}/log/vexfs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 640 vexfs vexfs
    postrotate
        systemctl reload vexfs || true
    endscript
}
EOF

%pre
# Create vexfs user and group
getent group vexfs >/dev/null || groupadd -r vexfs
getent passwd vexfs >/dev/null || \
    useradd -r -g vexfs -d %{_sharedstatedir}/vexfs -s /sbin/nologin \
    -c "VexFS service account" vexfs

%post
# Set proper ownership and permissions
chown vexfs:vexfs %{_sharedstatedir}/vexfs
chown vexfs:vexfs %{_localstatedir}/log/vexfs
chmod 750 %{_sharedstatedir}/vexfs
chmod 750 %{_localstatedir}/log/vexfs
chmod 640 %{_sysconfdir}/vexfs/vexfs.conf

# Enable and start systemd service
%systemd_post vexfs.service

%preun
%systemd_preun vexfs.service

%postun
%systemd_postun_with_restart vexfs.service

%files
%license LICENSE
%doc README.md
%{_bindir}/vexfs_server
%{_unitdir}/vexfs.service
%config(noreplace) %{_sysconfdir}/vexfs/vexfs.conf
%{_tmpfilesdir}/vexfs.conf
%{_sysconfdir}/logrotate.d/vexfs
%attr(750,vexfs,vexfs) %dir %{_sharedstatedir}/vexfs
%attr(750,vexfs,vexfs) %dir %{_localstatedir}/log/vexfs
%attr(750,root,vexfs) %dir %{_sysconfdir}/vexfs

%changelog
* Wed May 29 2024 VexFS Contributors <maintainers@vexfs.org> - 1.0.0-1
- Initial release of VexFS v1.0
- ChromaDB-compatible REST API
- High-performance vector similarity search
- Production-ready deployment features
- Comprehensive monitoring and logging
- Security hardening and best practices