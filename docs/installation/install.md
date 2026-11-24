# Installeren

Stacklane installeer je uit onze pakketbron op stacklane.io. Daarna zet `sudo stacklane setup` de rest klaar: de database, de webserver en je eerste beheerder.

## Wat je nodig hebt

- Een server met Debian 12 of nieuwer, of Ubuntu 24.04 of nieuwer, op amd64
- Minstens 1 vCPU en 1 GB geheugen
- Een domeinnaam die naar de server wijst, als je HTTPS wilt

PostgreSQL en Caddy hoef je niet vooraf te installeren. Het setup-commando vraagt of het dat voor je mag doen.

## De pakketbron toevoegen

De pakketten zijn ondertekend. Haal eerst de sleutel op en voeg dan de bron toe:

```sh
curl -fsSL https://stacklane.io/apt/stacklane.gpg \
  | sudo tee /usr/share/keyrings/stacklane.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/stacklane.gpg] \
  https://stacklane.io/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/stacklane.list
```

Controleer de vingerafdruk van de sleutel:

```sh
gpg --show-keys /usr/share/keyrings/stacklane.gpg
```

Die moet `3DE6 4DD4 93CE C7A9 01BC 5B4B 5AAC 95B7 CE6B 5075` zijn.

## Installeren

```sh
sudo apt update
sudo apt install stacklane
```

## Instellen

```sh
sudo stacklane setup
```

Het commando loopt stap voor stap door de installatie:

1. Het maakt de PostgreSQL-rol en -database `stacklane` aan en installeert PostgreSQL als het nog ontbreekt.
2. Het start de service en wacht tot die antwoordt.
3. Het vraagt op welk domein het panel komt en schrijft de Caddy-configuratie. Caddy haalt zelf een certificaat op.
4. Het maakt je beheerdersaccount aan.

Daarna open je het panel op je domein en log je in.

## Controleren

```sh
stacklane status
```

Je ziet of de service draait, welke versie het is en of de pakketbron goed staat.

## Wat er op je server komt

| Pad | Wat |
| --- | --- |
| `/usr/bin/stacklane` | Het programma: server en commando's |
| `/usr/share/stacklane/web` | Het klantpanel |
| `/etc/stacklane/stacklane.env` | [Instellingen](/docs/installation/configuration) |
| `/lib/systemd/system/stacklane.service` | De service |
| `/etc/caddy/stacklane.caddy` | De webserverconfiguratie, als je Caddy laat instellen |
