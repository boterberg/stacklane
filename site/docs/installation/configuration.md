# Configuratie

De service leest zijn instellingen uit `/etc/stacklane/stacklane.env`. Het pakket installeert dit bestand met werkende standaardwaarden en overschrijft je wijzigingen niet bij een update.

```sh
DATABASE_URL=postgres://stacklane@localhost/stacklane?host=/var/run/postgresql
BIND_ADDR=127.0.0.1:8080
COOKIE_SECURE=true
STATIC_DIR=/usr/share/stacklane/web
RUST_LOG=stacklane_server=info,tower_http=info
```

## Instellingen

| Variabele | Standaard | Betekenis |
| --- | --- | --- |
| `DATABASE_URL` | lokale socket, rol `stacklane` | Verbinding met PostgreSQL. Via de socket logt de systeemgebruiker `stacklane` in zonder wachtwoord. |
| `BIND_ADDR` | `127.0.0.1:8080` | Adres en poort waarop de server luistert. |
| `COOKIE_SECURE` | `true` | Stuur de sessiecookie alleen over HTTPS. Zet op `false` als je lokaal zonder HTTPS test. |
| `STATIC_DIR` | `/usr/share/stacklane/web` | Map met het klantpanel. Laat je hem leeg, dan serveert de server alleen de API. |
| `RUST_LOG` | `info` | Hoeveel de server logt, bijvoorbeeld `stacklane_server=debug`. |

Na een wijziging:

```sh
sudo systemctl restart stacklane
```

## Een externe database

Gebruik een gewone verbindings-URL met wachtwoord:

```sh
DATABASE_URL=postgres://stacklane:wachtwoord@db.example.com:5432/stacklane?sslmode=require
```

## Logboek

```sh
journalctl -u stacklane -f
```
