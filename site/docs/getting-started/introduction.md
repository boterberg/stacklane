# Introductie

Stacklane is een klantpanel voor hostingbedrijven. Je klanten beheren er hun diensten, stellen vragen aan je supportteam en bekijken hun facturen. Jij installeert het op je eigen server.

::: warning In ontwikkeling
Stacklane staat aan het begin. Bekijk de [releases](/releases/) voor wat er per versie bij is gekomen. Gebruik het nog niet voor klanten in productie.
:::

## Wat er nu werkt

| Onderdeel | Status |
| --- | --- |
| Account aanmaken, inloggen, wachtwoord vergeten | Beschikbaar |
| Dashboard met actieve tickets en diensten die verlopen | Beschikbaar |
| Tickets: aanmaken, reageren, sluiten, zoeken en filteren | Beschikbaar |
| Licht en donker thema, zoeken met <kbd>Ctrl</kbd>+<kbd>K</kbd> | Beschikbaar |
| Facturatie: abonnementen en facturen | In ontwikkeling |
| VPS, gameservers, domeinen en webhosting | Gepland |
| Account- en beveiligingsinstellingen, API-tokens | Gepland |

## Zo werkt het

Stacklane is één programma, `stacklane-server`, geschreven in Rust. Het bevat de API en serveert het klantpanel, een React-app. Alle gegevens staan in PostgreSQL. Voor HTTPS zet je er een reverse proxy voor, zoals Caddy.

```
browser ── https ──> Caddy ── http ──> stacklane-server ──> PostgreSQL
```

## Probeer het

- Bekijk de [demo](https://demo.stacklane.io) en maak daar een account aan.
- [Installeer Stacklane](/docs/installation/install) op je eigen server.
