# Meebouwen aan Stacklane

Alle code staat op [GitHub](https://github.com/boterberg/stacklane). Meld fouten en ideeën als issue, of open een pull request.

## Lokaal ontwikkelen

Je hebt Rust (stable), Node.js 22 en PostgreSQL nodig. Met podman of Docker start je de database zo:

```sh
podman compose up -d db
```

Start daarna de server en het panel:

```sh
cp server/.env.example server/.env
cd server && cargo run
```

```sh
cd web
npm install
npm run dev
```

Het panel draait op `http://localhost:5173` en stuurt `/api` door naar de server op poort 8080.

## Mappen

| Map | Inhoud |
| --- | --- |
| `server/` | Rust-server: API, migraties, sessies |
| `web/` | Klantpanel: React, TypeScript, Tailwind CSS |
| `site/` | Deze website en de documentatie, gebouwd met VitePress |
| `packaging/` | Debian-pakket, systemd-service, installatiescript |

## Voor je een pull request opent

```sh
cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd web && npm run lint && npm run build
```

Commitberichten volgen Conventional Commits, in kleine letters: `feat: …`, `fix(support): …`, `docs: …`.
