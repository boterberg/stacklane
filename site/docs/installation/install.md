# Installatie

Stacklane wordt geleverd als Debian-pakket (`.deb`) voor amd64. Het installatiescript zet de database klaar, installeert de laatste release en start de service.

## Ondersteunde systemen

| Systeem | Versie | Ondersteund |
| --- | --- | --- |
| Ubuntu | 26.04, 24.04 | ✅ |
| Debian | 13, 12 | ✅ |
| Andere Linux-distributies | | Alleen met eigen build |
| Windows | | ❌ |

## Benodigdheden

- Een server met minimaal 1 vCPU en 1 GB RAM
- PostgreSQL 15 of nieuwer (het script installeert het als het ontbreekt)
- Een domeinnaam die naar de server wijst, voor HTTPS

## Installeren met het script

```sh
curl -fsSLO https://raw.githubusercontent.com/boterberg/stacklane/main/packaging/install.sh
sudo sh install.sh
```

Het script:

1. installeert PostgreSQL als dat nog niet op de server staat;
2. maakt de databaserol en de database `stacklane` aan;
3. downloadt de laatste release en controleert die met `SHA256SUMS`;
4. installeert het pakket en wacht tot `/readyz` antwoordt.

Een specifieke versie installeer je met `sudo sh install.sh 0.0.1.0`.

## Handmatig installeren

```sh
sudo -u postgres createuser stacklane
sudo -u postgres createdb --owner stacklane stacklane

VERSION=0.0.1.0
curl -fsSLO https://github.com/boterberg/stacklane/releases/download/v$VERSION/stacklane_${VERSION}_amd64.deb
curl -fsSLO https://github.com/boterberg/stacklane/releases/download/v$VERSION/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
sudo apt install ./stacklane_${VERSION}_amd64.deb
```

De service maakt bij de eerste start zelf de tabellen aan.

## Controleren

```sh
systemctl status stacklane
curl http://127.0.0.1:8080/readyz
```

`/readyz` geeft `{"status":"ok","version":"…"}` terug zodra de server de database bereikt. Daarna [zet je de webserver op](/docs/installation/webserver).

## Wat het pakket installeert

| Pad | Inhoud |
| --- | --- |
| `/usr/bin/stacklane-server` | De server |
| `/usr/share/stacklane/web` | Het klantpanel |
| `/usr/share/stacklane/site` | Deze website |
| `/etc/stacklane/stacklane.env` | [Configuratie](/docs/installation/configuration) |
| `/lib/systemd/system/stacklane.service` | De systemd-service |
