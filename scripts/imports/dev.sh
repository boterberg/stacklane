#!/bin/sh
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)

case "${1:-up}" in
  up)
    podman rm -f stacklane-imports >/dev/null 2>&1 || true
    podman run -d --name stacklane-imports -e MARIADB_ROOT_PASSWORD=import -e MARIADB_DATABASE=whmcs \
      -e MARIADB_USER=import -e MARIADB_PASSWORD=import -p 127.0.0.1:3307:3306 \
      -v "$HERE/whmcs.sql:/docker-entrypoint-initdb.d/1-whmcs.sql:ro,Z" \
      -v "$HERE/paymenter.sql:/docker-entrypoint-initdb.d/2-paymenter.sql:ro,Z" \
      docker.io/library/mariadb:10.11 >/dev/null
    until podman exec stacklane-imports mariadb -uimport -pimport paymenter -e 'SELECT COUNT(*) FROM ticket_messages' >/dev/null 2>&1; do
      sleep 1
    done
    echo "WHMCS:     mysql://import:import@127.0.0.1:3307/whmcs"
    echo "Paymenter: mysql://import:import@127.0.0.1:3307/paymenter"
    ;;
  down)
    podman rm -f stacklane-imports >/dev/null 2>&1 || true
    echo "removed"
    ;;
  *)
    echo "usage: $0 up | down" >&2
    exit 1
    ;;
esac
