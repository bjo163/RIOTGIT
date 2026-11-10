#!/usr/bin/env bash
set -Eeuo pipefail

install -d -m 0755 /run/sshd /etc/ssh/hostkeys

HOST_KEY=/etc/ssh/hostkeys/ssh_host_ed25519_key
if [[ ! -s "$HOST_KEY" ]]; then
  ssh-keygen -q -t ed25519 -N "" -f "$HOST_KEY"
fi
chmod 0600 "$HOST_KEY"
chmod 0644 "$HOST_KEY.pub"

sshd -t -f /etc/ssh/sshd_config
exec /usr/sbin/sshd -D -e -f /etc/ssh/sshd_config
