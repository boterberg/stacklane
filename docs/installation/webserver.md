# Webserver en HTTPS

Stacklane luistert op `127.0.0.1:8080` en is dus niet direct van buiten bereikbaar. Een reverse proxy zet het panel op je domein en regelt HTTPS. `sudo stacklane setup` doet dat met Caddy; deze pagina is voor als je het zelf wilt regelen.

## Caddy

Het setup-commando schrijft `/etc/caddy/stacklane.caddy` en voegt `import stacklane.caddy` toe aan `/etc/caddy/Caddyfile`. Met de hand ziet dat er zo uit:

```txt
panel.jouwbedrijf.nl {
	encode zstd gzip
	reverse_proxy 127.0.0.1:8080
}
```

```sh
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy
```

Caddy vraagt het certificaat zelf aan zodra het domein naar je server wijst.

## Nginx of een andere webserver

Het kan met elke reverse proxy die verzoeken doorstuurt naar `127.0.0.1:8080` en de `Host`-header laat staan. Voor nginx:

```nginx
server {
    server_name panel.jouwbedrijf.nl;
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Het certificaat regel je dan zelf, bijvoorbeeld met certbot.

::: tip Cookies over HTTPS
Laat `COOKIE_SECURE=true` staan zodra het panel via HTTPS bereikbaar is. De browser stuurt de sessiecookie dan nooit over een onversleutelde verbinding.
:::
