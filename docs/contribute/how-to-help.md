# Meebouwen

De code staat op [GitHub](https://github.com/boterberg/stacklane). Fouten en ideeën kun je daar melden; een pull request is ook welkom.

## Lokaal draaien

Je hebt Rust, Node.js 22 en podman of Docker nodig. De database start je met een script:

```sh
sh scripts/dev-db.sh
cp .env.example .env
cargo run
```

Het panel start je in een tweede terminal:

```sh
cd web
npm install
npm run dev
```

Het panel staat dan op `http://localhost:5173` en praat met de server op poort 8080. Deze website draait met `npm run dev` in de map `docs`.

## Mappen

| Map | Inhoud |
| --- | --- |
| `src/` | De Rust-server en het commando `stacklane` |
| `migrations/` | Databasemigraties |
| `web/` | Het klantpanel in React en Tailwind CSS |
| `docs/` | Deze website, gebouwd met VitePress |
| `packaging/` | Het Debian-pakket en de systemd-units |
| `deploy/` | De pakketbron en de demo op stacklane.io |
| `scripts/` | Hulpscripts voor ontwikkelen |

## Voor je iets instuurt

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cd web && npm run lint && npm run build
```

Commitberichten schrijven we kort en in kleine letters, zoals `feat: …`, `fix(support): …` of `docs: …`.
