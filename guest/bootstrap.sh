#!/bin/bash
set -eu
mkdir -p /mnt/research
mkdir -p /lib/modules/5.4.0-216-generic/extra
touch /etc/cloud/cloud-init.disabled
rm -f /root/.ssh/authorized_keys
mkdir -p /etc/ssh/sshd_config.d
cat > /etc/ssh/sshd_config.d/10-gxfp51b7.conf <<'EOF'
PermitRootLogin no
PasswordAuthentication no
KbdInteractiveAuthentication no
AllowUsers ubuntu
AuthenticationMethods publickey
EOF
install -m 0644 /home/ubuntu/legacy-driver/isgx.ko /lib/modules/5.4.0-216-generic/extra/gxfp51b7-isgx.ko
cat > /usr/local/sbin/gxfp51b7-guest-prepare <<'EOF'
#!/bin/bash
set -eu
modprobe 9pnet_virtio
mountpoint -q /mnt/research || mount -t 9p -o trans=virtio,version=9p2000.L,ro research /mnt/research
mount -o remount,exec /dev
test -e /dev/isgx || insmod /lib/modules/5.4.0-216-generic/extra/gxfp51b7-isgx.ko
EOF
chmod 0755 /usr/local/sbin/gxfp51b7-guest-prepare
cat > /etc/systemd/system/gxfp51b7-guest.service <<'EOF'
[Unit]
Description=Prepare isolated fingerprint enclaves
After=local-fs.target
Before=ssh.service
[Service]
Type=oneshot
ExecStart=/usr/local/sbin/gxfp51b7-guest-prepare
RemainAfterExit=yes
[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable gxfp51b7-guest.service
chown root:root /home/ubuntu/legacy_load
chmod 0755 /home/ubuntu/legacy_load
