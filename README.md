<br>
<p align="center">
  <a href="https://stacklane.io">
    <img alt="Stacklane" src="logo/stacklane-symbol.svg" width="72">
  </a>
</p>
<h1 align="center">
  Stacklane
</h1>

<div align="center">
  <h3>A customer panel for hosting companies</h3>
  <p>Your customers log in, see what they have with you and talk to your support team. You run it on your own server and install it with apt.</p>
</div>

<h4 align="center">
  <a href="https://stacklane.io">Website</a> ·
  <a href="https://stacklane.io/docs/getting-started/introduction">Documentation</a> ·
  <a href="https://demo.stacklane.io/login">Live demo</a> ·
  <a href="https://stacklane.io/releases/">Releases</a>
</h4>

<div align="center">

  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
  [![CI](https://github.com/boterberg/stacklane/actions/workflows/ci.yml/badge.svg)](https://github.com/boterberg/stacklane/actions/workflows/ci.yml)

</div>

<div align="center">
  <img alt="The Stacklane dashboard" src="docs/public/panel-dashboard.webp">
</div>

## Getting started

#### Installation and documentation

The [documentation](https://stacklane.io/docs/installation/install) walks you through it. On Debian 12+ or Ubuntu 24.04+:

```sh
curl -fsSL https://stacklane.io/apt/stacklane.gpg | sudo tee /usr/share/keyrings/stacklane.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/stacklane.gpg] https://stacklane.io/apt stable main" | sudo tee /etc/apt/sources.list.d/stacklane.list
sudo apt update && sudo apt install stacklane
sudo stacklane setup
```

`stacklane setup` creates the database, writes the Caddy config for your domain and asks for your first admin account. Later, `sudo stacklane update` backs up the database and installs the new version.

#### Requirements

- Debian 12 or newer, or Ubuntu 24.04 or newer, on amd64
- PostgreSQL 15 or newer (setup installs it if it is missing)
- A domain pointing at the server, for HTTPS

## What is Stacklane?

Stacklane is the part of a hosting company your customers actually see. Today they can sign up, check their dashboard and open support tickets, which your team answers from a shared inbox. Billing, VPS, game servers, domains and web hosting are next; the [roadmap](https://stacklane.io/#roadmap) has the order.

It is a single Rust binary with the React panel built in. It runs as a systemd service, keeps its data in PostgreSQL and ships as a Debian package from our own signed repository.

Stacklane is available under the MIT license.

## Development

```sh
sh scripts/dev-db.sh
cp .env.example .env
cargo run
```

```sh
cd web && npm install && npm run dev
```

The panel runs on http://localhost:5173. The website lives in `docs/`. More in the [contributor guide](https://stacklane.io/docs/contribute/how-to-help).
