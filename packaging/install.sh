#!/bin/sh
set -eu

REPO=boterberg/stacklane
VERSION=${1:-latest}

[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }

if ! command -v psql >/dev/null; then
  apt-get update
  apt-get install -y postgresql
fi

if ! su postgres -c "psql -Atc \"SELECT 1 FROM pg_roles WHERE rolname = 'stacklane'\"" | grep -q 1; then
  su postgres -c "createuser stacklane"
fi
if ! su postgres -c "psql -Atc \"SELECT 1 FROM pg_database WHERE datname = 'stacklane'\"" | grep -q 1; then
  su postgres -c "createdb --owner stacklane stacklane"
fi

if [ "$VERSION" = latest ]; then
  url=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep -o '"browser_download_url": *"[^"]*_amd64\.deb"' | cut -d'"' -f4)
else
  url="https://github.com/$REPO/releases/download/v$VERSION/stacklane_${VERSION}_amd64.deb"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fsSL -o "$tmp/stacklane.deb" "$url"
curl -fsSL -o "$tmp/SHA256SUMS" "$(dirname "$url")/SHA256SUMS"
(cd "$tmp" && grep "$(basename "$url")" SHA256SUMS | sed "s|$(basename "$url")|stacklane.deb|" | sha256sum -c -)
apt-get install -y "$tmp/stacklane.deb"

for attempt in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8080/readyz >/dev/null 2>&1; then
    curl -fsS http://127.0.0.1:8080/readyz
    echo
    exit 0
  fi
  sleep 1
done
echo "stacklane did not become ready, see: journalctl -u stacklane" >&2
exit 1
