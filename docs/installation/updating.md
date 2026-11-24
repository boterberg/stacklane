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

Een versie heeft vier getallen, zoals `0.0.1.0`. Wat er per versie verandert, staat bij de [releases](/releases/).
