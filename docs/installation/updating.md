# Bijwerken

```sh
sudo stacklane update
```

Dat is alles. Het commando:

1. kijkt in de pakketbron of er een nieuwere versie is;
2. maakt een back-up van de database in `/var/backups/stacklane`;
3. installeert de nieuwe versie met apt;
4. wacht tot de server weer antwoordt en toont de nieuwe versie.

De server voert de databasemigraties zelf uit bij het starten. Draai je al de nieuwste versie, dan stopt het commando na stap 1.

## Vanuit het panel

Stacklane kijkt elke zes uur in de pakketbron of er een nieuwe versie is. Is die er, dan zien beheerders linksonder in het menu een oranje driehoek met de nieuwe versie, en op het dashboard een knop **Nu bijwerken**.

De knop doet precies hetzelfde als `sudo stacklane update`, met back-up en al. De panelservice zelf heeft geen rootrechten. Hij schrijft alleen een verzoek naar `/var/lib/stacklane/update/request`, waarna de systemd-unit `stacklane-update.path` het commando als root start. Het commando leest dat verzoek niet en raakt niets aan in de map van het panel: welke versie het installeert, haalt het uit de ondertekende pakketbron, en de voortgang schrijft het naar `/var/lib/stacklane-update`, een map waarin alleen root kan schrijven. Het pakket zet de unit aan bij de eerste installatie en laat hem bij een update uit als je hem zelf hebt uitgezet.

Tijdens het herstarten is het panel een halve minuut niet bereikbaar. De voortgang staat in het panel, en volledig in het logboek:

```sh
journalctl -u stacklane-update -n 100
```

Wil je niet dat beheerders vanuit het panel kunnen bijwerken, zet de unit dan uit:

```sh
sudo systemctl disable --now stacklane-update.path
```

De melding blijft dan staan, met het commando om zelf bij te werken.

## Als er iets misgaat

Start de nieuwe versie niet, dan zegt het commando dat en noemt het de plek van de back-up. Kijk eerst in het logboek:

```sh
journalctl -u stacklane -n 100
```

Een back-up terugzetten:

```sh
sudo systemctl stop stacklane
sudo -u postgres pg_restore --clean --dbname stacklane /var/backups/stacklane/<bestand>.dump
sudo systemctl start stacklane
```

## Versienummers

Een versie heeft vier getallen, zoals `1.0.0.0`. Wat er per versie verandert, staat bij de [releases](/releases/).
