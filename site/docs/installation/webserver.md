# Webserver en HTTPS

`stacklane-server` luistert standaard alleen op `127.0.0.1:8080`. Zet er een reverse proxy voor die HTTPS afhandelt. We raden Caddy aan: die haalt automatisch een certificaat op.

## Caddy installeren

```sh
sudo apt install caddy
```

## Configuratie

Vervang `panel.example.com` door je eigen domein en zet dit in `/etc/caddy/Caddyfile`:

```txt
panel.example.com {
	encode zstd gzip
	reverse_proxy 127.0.0.1:8080
}
```

Wil je ook deze website hosten, voeg dan toe:

```txt
example.com {
	encode zstd gzip
	root * /usr/share/stacklane/site
	try_files {path} {path}.html {path}/index.html
	file_server
}
```

Controleer de configuratie en laad hem opnieuw:

```sh
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

::: tip Cookies
Laat `COOKIE_SECURE=true` staan zodra het panel via HTTPS bereikbaar is. Dan stuurt de browser de sessiecookie nooit over een onversleutelde verbinding.
:::

## Een andere webserver

Elke reverse proxy werkt, zolang die alle verzoeken doorstuurt naar `127.0.0.1:8080` en de `Host`-header behoudt. Het panel is een single-page app: de server stuurt zelf `index.html` terug voor paden die geen bestand zijn.
