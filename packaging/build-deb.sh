#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

VERSION="" BIN="" WEB="" SITE="" OUT=dist ARCH=amd64
while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION=$2; shift 2 ;;
    --bin) BIN=$2; shift 2 ;;
    --web) WEB=$2; shift 2 ;;
    --site) SITE=$2; shift 2 ;;
    --arch) ARCH=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

[ -n "$VERSION" ] && [ -x "$BIN" ] && [ -f "$WEB/index.html" ] && [ -f "$SITE/index.html" ] || {
  echo "usage: $0 --version X --bin <stacklane-server> --web <web/dist> --site <site/dist> [--arch amd64] [--out dist]" >&2
  exit 1
}
echo "$VERSION" | grep -Eq '^[0-9][0-9A-Za-z.+~-]*$' || { echo "invalid version: $VERSION" >&2; exit 1; }

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
chmod 0755 "$root"

install -d -m 0755 "$root/DEBIAN" "$root/usr/bin" "$root/usr/share/stacklane" "$root/usr/share/doc/stacklane" \
  "$root/lib/systemd/system" "$root/etc/stacklane"

install -m 0755 "$BIN" "$root/usr/bin/stacklane-server"
cp -R "$WEB" "$root/usr/share/stacklane/web"
cp -R "$SITE" "$root/usr/share/stacklane/site"
find "$root/usr/share/stacklane" -type d -exec chmod 0755 {} +
find "$root/usr/share/stacklane" -type f -exec chmod 0644 {} +
printf '%s\n' "$VERSION" > "$root/usr/share/stacklane/VERSION"
chmod 0644 "$root/usr/share/stacklane/VERSION"

install -m 0644 packaging/systemd/stacklane.service "$root/lib/systemd/system/stacklane.service"
install -m 0640 packaging/etc/stacklane.env "$root/etc/stacklane/stacklane.env"
install -m 0644 packaging/Caddyfile.example README.md "$root/usr/share/doc/stacklane/"

size=$(du -sk --exclude=DEBIAN "$root" | cut -f1)
sed -e "s/@VERSION@/$VERSION/" -e "s/@SIZE@/$size/" -e "s/@ARCH@/$ARCH/" packaging/debian/control.in > "$root/DEBIAN/control"
echo "/etc/stacklane/stacklane.env" > "$root/DEBIAN/conffiles"
for script in postinst prerm postrm; do
  install -m 0755 "packaging/debian/$script" "$root/DEBIAN/$script"
done

mkdir -p "$OUT"
deb="$OUT/stacklane_${VERSION}_${ARCH}.deb"
dpkg-deb --root-owner-group -Zxz --build "$root" "$deb" >/dev/null
echo "built $deb"
dpkg-deb --info "$deb" | sed -n '/Package:/,/Description:/p'
