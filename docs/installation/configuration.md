# Instellingen

De service leest zijn instellingen uit `/etc/stacklane/stacklane.env`. Bij een update blijven jouw wijzigingen in dat bestand staan.

```sh
DATABASE_URL=postgres://stacklane@localhost/stacklane?host=/var/run/postgresql
BIND_ADDR=127.0.0.1:8080
COOKIE_SECURE=true
STATIC_DIR=/usr/share/stacklane/web
RUST_LOG=stacklane=info,tower_http=info
```

| Variabele | Standaard | Wat het doet |
| --- | --- | --- |
| `DATABASE_URL` | lokale socket | Verbinding met PostgreSQL. Via de socket logt de systeemgebruiker `stacklane` in zonder wachtwoord. |
| `BIND_ADDR` | `127.0.0.1:8080` | Adres en poort van de server. |
| `COOKIE_SECURE` | `true` | Stuurt de sessiecookie alleen over HTTPS. Zet het op `false` als je lokaal zonder HTTPS test. |
| `STATIC_DIR` | `/usr/share/stacklane/web` | Map met het klantpanel. Laat je hem weg, dan levert de server alleen de API. |
| `RUST_LOG` | `info` | Hoeveel er gelogd wordt, bijvoorbeeld `stacklane=debug`. |
| `PUBLIC_URL` | leeg | Het adres van het panel, zoals `https://panel.jouwbedrijf.nl`. Komt in de links in e-mails. `stacklane setup` vult het in. |
| `SECRET_KEY` | wordt aangemaakt | Sleutel waarmee Stacklane de API-sleutels van je koppelingen versleutelt. Het pakket maakt hem bij de installatie. Raak je hem kwijt, dan moet je die sleutels opnieuw invoeren. |
| `SMTP_URL` | leeg | Mailserver voor bijvoorbeeld wachtwoordherstel, zoals `smtps://gebruiker:wachtwoord@smtp.jouwbedrijf.nl:465`. Zonder deze instelling verstuurt Stacklane geen e-mail. |
| `MAIL_FROM` | leeg | Afzender, zoals `Jouw Hosting <noreply@jouwbedrijf.nl>`. |
| `UPDATE_CHECK_URL` | pakketbron van stacklane.io | Waar Stacklane kijkt of er een nieuwe versie is. Leeg laten zet de controle uit. |
| `DEMO_MODE` | uit | Inloggen in één klik als klant of beheerder, en `stacklane demo-reset` mag de database leegmaken. Zet dit nooit aan op een server met echte klanten. |

Na een wijziging:

```sh
sudo systemctl restart stacklane
```

## Een database op een andere server

```sh
DATABASE_URL=postgres://stacklane:wachtwoord@db.jouwbedrijf.nl:5432/stacklane?sslmode=require
```

## Logboek

```sh
journalctl -u stacklane -f
```
