#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."

HOST=${STACKLANE_SITE_HOST:-ubuntu@5.252.26.94}
KEY=${STACKLANE_SITE_KEY:-$HOME/.ssh/bcode_prod}

(cd docs && npm ci --no-audit --no-fund >/dev/null && npm run build >/dev/null)
tar -C docs/dist -czf - . | ssh -i "$KEY" "$HOST" '
  set -eu
  next=$(mktemp -d /tmp/stacklane-site.XXXXXX)
  tar -C "$next" -xzf -
  sudo chmod -R a+rX "$next"
  sudo rm -rf /srv/stacklane-site.previous
  [ -d /srv/stacklane-site ] && sudo mv /srv/stacklane-site /srv/stacklane-site.previous
  sudo mv "$next" /srv/stacklane-site
  echo "site deployed: $(find /srv/stacklane-site -type f | wc -l) files"
'
