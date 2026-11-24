#!/bin/sh
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)

case "${1:-up}" in
  up)
    podman rm -f stacklane-whmcs >/dev/null 2>&1 || true
    podman run -d --name stacklane-whmcs -e MARIADB_ROOT_PASSWORD=whmcs -e MARIADB_DATABASE=whmcs \
      -e MARIADB_USER=whmcs -e MARIADB_PASSWORD=whmcs -p 127.0.0.1:3307:3306 \
      -v "$HERE/fixture.sql:/docker-entrypoint-initdb.d/fixture.sql:ro,Z" docker.io/library/mariadb:10.11 >/dev/null
    until podman exec stacklane-whmcs mariadb -uwhmcs -pwhmcs whmcs -e 'SELECT COUNT(*) FROM tblaccounts' >/dev/null 2>&1; do
      sleep 1
    done
    echo "WHMCS test database: mysql://whmcs:whmcs@127.0.0.1:3307/whmcs"
    ;;
  down)
    podman rm -f stacklane-whmcs >/dev/null 2>&1 || true
    echo "removed"
    ;;
  *)
    echo "usage: $0 up | down" >&2
    exit 1
    ;;
esac
