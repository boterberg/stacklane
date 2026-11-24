#!/bin/sh
set -eu
cd "$(dirname "$0")"

HOST=${STACKLANE_SITE_HOST:-ubuntu@5.252.26.94}
KEY=${STACKLANE_SITE_KEY:-$HOME/.ssh/bcode_prod}

tar -czf - stats.py stacklane-stats.service stacklane-stats.timer | ssh -i "$KEY" "$HOST" '
  set -eu
  work=$(mktemp -d)
  tar -C "$work" -xzf -
  sudo install -D -m 0755 "$work/stats.py" /usr/local/lib/stacklane-stats/stats.py
  sudo install -m 0644 "$work/stacklane-stats.service" "$work/stacklane-stats.timer" /etc/systemd/system/
  sudo install -d -m 0755 /srv/stacklane-stats
  rm -rf "$work"
  sudo systemctl daemon-reload
  sudo systemctl enable --now stacklane-stats.timer
  sudo systemctl start stacklane-stats.service
  cat /srv/stacklane-stats/stats.json; echo
'
