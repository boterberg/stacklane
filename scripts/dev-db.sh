#!/bin/sh
set -eu

NAME=stacklane-db
ENGINE=$(command -v podman || command -v docker)

if "$ENGINE" container exists "$NAME" 2>/dev/null || "$ENGINE" inspect "$NAME" >/dev/null 2>&1; then
  "$ENGINE" start "$NAME" >/dev/null
else
  "$ENGINE" run -d --name "$NAME" \
    -e POSTGRES_USER=stacklane -e POSTGRES_PASSWORD=stacklane -e POSTGRES_DB=stacklane \
    -p 127.0.0.1:5432:5432 -v stacklane-db:/var/lib/postgresql/data \
    docker.io/library/postgres:17-alpine >/dev/null
fi

until "$ENGINE" exec "$NAME" pg_isready -U stacklane >/dev/null 2>&1; do
  sleep 1
done
echo "postgres://stacklane:stacklane@127.0.0.1:5432/stacklane"
