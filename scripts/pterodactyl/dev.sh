#!/bin/sh
set -eu

HERE=$(cd "$(dirname "$0")" && pwd)
BASE=${STACKLANE_PTERO_DIR:-$HOME/.local/share/stacklane-ptero}
PANEL_IMAGE=ghcr.io/pterodactyl/panel:v1.15.2
WINGS_VERSION=v1.13.5
WINGS_SHA256=231134aa6d5aa02aac4f6384b50c76860637858cbc056268b9f96ac42f7f5393
PANEL_URL=http://127.0.0.1:8180
WINGS_PORT=8081
SOCKET=$BASE/run/podman.sock

log() {
  printf '  %s\n' "$*"
}

panel() {
  podman exec ptero-panel "$@"
}

secrets() {
  mkdir -p "$BASE/run" "$BASE/data/mariadb" "$BASE/data/redis" "$BASE/data/panel-var" "$BASE/data/panel-logs" "$BASE/bin" "$BASE/wings/lib" "$BASE/wings/log" "$BASE/wings/tmp"
  if [ ! -s "$BASE/secrets.env" ]; then
    umask 077
    {
      echo "DB_PASSWORD=$(openssl rand -hex 16)"
      echo "DB_ROOT_PASSWORD=$(openssl rand -hex 16)"
      echo "APP_KEY=base64:$(openssl rand -base64 32)"
      echo "ADMIN_PASSWORD=Adm1n-$(openssl rand -hex 8)"
    } > "$BASE/secrets.env"
  fi
  . "$BASE/secrets.env"
}

containers() {
  podman network exists pterodactyl_nw || podman network create --subnet 172.18.0.0/16 --gateway 172.18.0.1 pterodactyl_nw >/dev/null
  podman pod exists ptero || podman pod create --name ptero --network "pasta:-T,$WINGS_PORT" -p 127.0.0.1:8180:80 >/dev/null

  printf '[client]\nskip-ssl\n' > "$BASE/mysql-client.cnf"

  podman container exists ptero-db || podman run -d --pod ptero --name ptero-db \
    -e MARIADB_ROOT_PASSWORD="$DB_ROOT_PASSWORD" -e MARIADB_DATABASE=panel \
    -e MARIADB_USER=pterodactyl -e MARIADB_PASSWORD="$DB_PASSWORD" \
    -v "$BASE/data/mariadb:/var/lib/mysql:Z" docker.io/library/mariadb:10.11 >/dev/null 2>&1
  podman container exists ptero-redis || podman run -d --pod ptero --name ptero-redis \
    -v "$BASE/data/redis:/data:Z" docker.io/library/redis:7-alpine >/dev/null 2>&1
  podman container exists ptero-panel || podman run -d --pod ptero --name ptero-panel \
    -e APP_URL="$PANEL_URL" -e APP_TIMEZONE=UTC -e APP_SERVICE_AUTHOR=admin@ptero.test -e APP_KEY="$APP_KEY" \
    -e APP_ENVIRONMENT_ONLY=false -e TRUSTED_PROXIES='*' \
    -e DB_HOST=127.0.0.1 -e DB_PORT=3306 -e DB_DATABASE=panel -e DB_USERNAME=pterodactyl -e DB_PASSWORD="$DB_PASSWORD" \
    -e CACHE_DRIVER=redis -e SESSION_DRIVER=redis -e QUEUE_DRIVER=redis -e REDIS_HOST=127.0.0.1 \
    -e RECAPTCHA_ENABLED=false -e MAIL_DRIVER=log \
    -v "$BASE/data/panel-var:/app/var:Z" -v "$BASE/data/panel-logs:/app/storage/logs:Z" \
    -v "$BASE/mysql-client.cnf:/etc/my.cnf.d/skip-ssl.cnf:ro,Z" \
    "$PANEL_IMAGE" >/dev/null 2>&1
  podman start ptero-db ptero-redis ptero-panel >/dev/null 2>&1

  log "waiting for the panel on $PANEL_URL"
  for _ in $(seq 1 120); do
    if curl -fsS -o /dev/null "$PANEL_URL/auth/login"; then
      return 0
    fi
    sleep 2
  done
  echo "the panel did not come up, see: podman logs ptero-panel" >&2
  exit 1
}

bootstrap() {
  [ -s "$BASE/keys.env" ] && return 0
  log "creating admin, location and node"
  panel php artisan p:user:make --email=admin@ptero.test --username=admin --name-first=Ptero --name-last=Admin --password="$ADMIN_PASSWORD" --admin=1 >/dev/null
  panel php artisan p:location:make --short=local --long="Local development" >/dev/null
  panel php artisan p:node:make --name=local-wings --description="Wings on this machine" --locationId=1 \
    --fqdn=127.0.0.1 --public=1 --scheme=http --proxy=0 --maintenance=0 \
    --maxMemory=8192 --overallocateMemory=0 --maxDisk=40960 --overallocateDisk=0 --uploadSize=100 \
    --daemonListeningPort=$WINGS_PORT --daemonSFTPPort=2022 --daemonBase="$BASE/wings/lib/volumes" >/dev/null
  podman cp "$HERE/make-keys.php" ptero-panel:/tmp/make-keys.php
  panel sh -c 'php artisan tinker --execute="require \"/tmp/make-keys.php\";"' | grep '^PTERODACTYL_' > "$BASE/keys.env"
  . "$BASE/keys.env"
  curl -fsS -X POST "$PANEL_URL/api/application/nodes/1/allocations" \
    -H "Authorization: Bearer $PTERODACTYL_APPLICATION_KEY" -H 'Accept: application/json' -H 'Content-Type: application/json' \
    -d '{"ip":"127.0.0.1","alias":"localhost","ports":["25565-25575"]}' >/dev/null
}

wings() {
  if [ ! -x "$BASE/bin/wings" ]; then
    log "downloading Wings $WINGS_VERSION"
    curl -fsSL -o "$BASE/bin/wings.download" "https://github.com/pterodactyl/wings/releases/download/$WINGS_VERSION/wings_linux_amd64"
    echo "$WINGS_SHA256  $BASE/bin/wings.download" | sha256sum -c - >/dev/null
    chmod +x "$BASE/bin/wings.download"
    mv "$BASE/bin/wings.download" "$BASE/bin/wings"
  fi

  printf '#!/bin/sh\nexec python3 -I %s "$@"\n' "$HERE/crun-wrapper.py" > "$BASE/bin/crun"
  chmod +x "$BASE/bin/crun"
  printf '[containers]\nlabel = false\n\n[engine.runtimes]\ncrun = ["%s"]\n' "$BASE/bin/crun" > "$BASE/containers.conf"

  if [ ! -S "$SOCKET" ] || ! curl -fsS --unix-socket "$SOCKET" http://d/_ping >/dev/null 2>&1; then
    rm -f "$SOCKET"
    CONTAINERS_CONF_OVERRIDE="$BASE/containers.conf" setsid podman system service --time=0 "unix://$SOCKET" > "$BASE/run/podman.log" 2>&1 < /dev/null &
    echo $! > "$BASE/run/podman.pid"
    for _ in $(seq 1 20); do
      curl -fsS --unix-socket "$SOCKET" http://d/_ping >/dev/null 2>&1 && break
      sleep 0.5
    done
  fi

  panel php artisan p:node:configuration 1 --format=yml > "$BASE/wings/panel-config.yml"
  python3 - "$BASE" "$WINGS_PORT" "$PANEL_URL" <<'PY'
import sys

import yaml

base, port, panel = sys.argv[1], int(sys.argv[2]), sys.argv[3]
with open(f"{base}/wings/panel-config.yml") as handle:
    config = yaml.safe_load(handle)
config["api"].update({"host": "127.0.0.1", "port": port})
config["system"] = {
    "root_directory": f"{base}/wings/lib",
    "log_directory": f"{base}/wings/log",
    "data": f"{base}/wings/lib/volumes",
    "archive_directory": f"{base}/wings/lib/archives",
    "backup_directory": f"{base}/wings/lib/backups",
    "tmp_directory": f"{base}/wings/tmp",
    "username": "pterodactyl",
    "timezone": "UTC",
    "user": {"rootless": {"enabled": True, "container_uid": 0, "container_gid": 0}},
    "passwd": {"enable": False, "directory": f"{base}/wings/passwd"},
    "machine_id": {"enable": False, "directory": f"{base}/wings/machine-id"},
    "enable_log_rotate": False,
    "sftp": {"bind_address": "127.0.0.1", "bind_port": 2022},
}
config["docker"] = {
    "network": {
        "interface": "127.0.0.1",
        "name": "pterodactyl_nw",
        "network_mode": "pterodactyl_nw",
        "driver": "bridge",
        "interfaces": {"v4": {"subnet": "172.18.0.0/16", "gateway": "172.18.0.1"}},
    },
    "log_config": {"type": "json-file", "config": {"max-size": "5m", "max-file": "1", "compress": "false", "mode": "non-blocking"}},
}
config["remote"] = panel
config["allowed_origins"] = [
    panel,
    "http://localhost:8180",
    "http://127.0.0.1:8080",
    "http://localhost:8080",
    "http://localhost:5173",
    "http://127.0.0.1:5173",
]
with open(f"{base}/wings/config.yml", "w") as handle:
    yaml.safe_dump(config, handle, sort_keys=False)
PY

  if [ -s "$BASE/run/wings.pid" ] && kill -0 "$(cat "$BASE/run/wings.pid")" 2>/dev/null; then
    kill "$(cat "$BASE/run/wings.pid")"
    sleep 1
  fi
  (cd "$BASE/run" && DOCKER_HOST="unix://$SOCKET" setsid "$BASE/bin/wings" --config "$BASE/wings/config.yml" > "$BASE/run/wings.log" 2>&1 < /dev/null &
    echo $! > "$BASE/run/wings.pid")
  for _ in $(seq 1 30); do
    if curl -fsS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$WINGS_PORT/api/system" 2>/dev/null | grep -q 401; then
      return 0
    fi
    curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$WINGS_PORT/api/system" 2>/dev/null | grep -q 401 && return 0
    sleep 1
  done
  echo "Wings did not start, see $BASE/run/wings.log" >&2
  exit 1
}

up() {
  secrets
  containers
  bootstrap
  wings
  . "$BASE/keys.env"
  echo
  log "Pterodactyl panel  $PANEL_URL  (admin@ptero.test / $ADMIN_PASSWORD)"
  log "Wings              http://127.0.0.1:$WINGS_PORT"
  log "API keys           $BASE/keys.env"
}

down() {
  for pid in "$BASE/run/wings.pid" "$BASE/run/podman.pid"; do
    if [ -s "$pid" ]; then
      kill "$(cat "$pid")" 2>/dev/null || true
      rm -f "$pid"
    fi
  done
  podman ps -a --filter label=Service=Pterodactyl --format '{{.ID}}' | xargs -r podman rm -f >/dev/null
  podman pod rm -f ptero >/dev/null 2>&1 || true
  podman network rm pterodactyl_nw >/dev/null 2>&1 || true
  if [ "${1:-}" = "--purge" ]; then
    podman unshare rm -rf "$BASE"
    log "removed $BASE"
  fi
  log "Pterodactyl stopped"
}

case "${1:-up}" in
  up) up ;;
  down) shift; down "$@" ;;
  *) echo "usage: $0 up | down [--purge]" >&2; exit 1 ;;
esac
