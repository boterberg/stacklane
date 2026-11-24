# Stacklane

Customer panel for hosting: game servers, VPS, domains, web hosting, support and billing.

## Stack

- `server/`: Rust, axum, sqlx, PostgreSQL
- `web/`: React, TypeScript, Vite, Tailwind CSS, Headless UI, Heroicons

## Development

```sh
podman compose up -d db
cp server/.env.example server/.env
cd server && cargo run
```

```sh
cd web
npm install
npm run dev
```

The panel runs on http://localhost:5173 and proxies `/api` to the server on port 8080.

## Releases

A tag `v0.0.1.0` (four numbers) builds the web panel, the website and the server, packages them as `stacklane_<version>_amd64.deb` and publishes a GitHub release with `SHA256SUMS`.

## Installing on a server

```sh
curl -fsSLO https://raw.githubusercontent.com/boterberg/stacklane/main/packaging/install.sh
sudo sh install.sh
```

The script creates the PostgreSQL role and database, installs the latest release and waits until `/readyz` answers. Configuration lives in `/etc/stacklane/stacklane.env`; `packaging/Caddyfile.example` shows the reverse proxy for the panel and the website.
