# Meebouwen aan Stacklane

Alle code staat op [GitHub](https://github.com/boterberg/stacklane). Meld fouten en ideeën als issue, of open een pull request.

## Lokaal ontwikkelen

Je hebt Rust (stable), Node.js 22 en podman of Docker nodig. Start de database en de server:

```sh
sh scripts/dev-db.sh
cp .env.example .env
cargo run
```

Start in een tweede terminal het panel:

```sh
cd web
npm install
npm run dev
```

Het panel draait op `http://localhost:5173` en stuurt `/api` door naar de server op poort 8080.

## Mappen

| Map | Inhoud |
| --- | --- |
| `src/` | Rust-server: API, sessies, demo |
| `migrations/` | Databasemigraties, worden bij elke start toegepast |
| `web/` | Klantpanel: React, TypeScript, Tailwind CSS |
| `docs/` | Deze website en de documentatie, gebouwd met VitePress |
| `packaging/` | Debian-pakket, systemd-units, installatiescript |
| `deploy/` | Configuratie van de demo op demo.stacklane.io |
| `scripts/` | Hulpscripts voor ontwikkeling |

## Voor je een pull request opent

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd web && npm run lint && npm run build
```

Commitberichten volgen Conventional Commits, in kleine letters: `feat: …`, `fix(support): …`, `docs: …`.
