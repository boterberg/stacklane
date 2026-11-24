# Updaten

Een update is een nieuwere versie van het pakket installeren. De server voert bij het starten zelf de databasemigraties uit.

::: warning Maak eerst een back-up
```sh
sudo -u postgres pg_dump --format=custom stacklane > stacklane-$(date +%F).dump
```
:::

## Met het script

```sh
sudo sh install.sh
```

Zonder versienummer installeert het script de laatste release.

## Handmatig

```sh
VERSION=0.0.1.0
curl -fsSLO https://github.com/boterberg/stacklane/releases/download/v$VERSION/stacklane_${VERSION}_amd64.deb
curl -fsSLO https://github.com/boterberg/stacklane/releases/download/v$VERSION/SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
sudo apt install ./stacklane_${VERSION}_amd64.deb
curl http://127.0.0.1:8080/readyz
```

`/readyz` toont de geïnstalleerde versie.

## Versienummers

Stacklane gebruikt vier getallen: `v0.0.1.0`. Wat er per versie verandert, staat bij de [releases](/releases/).
