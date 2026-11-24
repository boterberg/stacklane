# Introductie

Stacklane is het panel waar de klanten van je hostingbedrijf inloggen. Ze zien er hun overzicht en stellen vragen aan je supportteam. Je draait het zelf, op een server die je al hebt of een nieuwe.

::: warning Vroege versie
Stacklane is net begonnen. Kijk bij de [releases](/releases/) wat er in de huidige versie zit, en gebruik het nog niet voor betalende klanten.
:::

## Wat er nu in zit

| Onderdeel | Sinds |
| --- | --- |
| Accounts: registreren, inloggen en uitloggen | 0.0.1.0 |
| Dashboard met openstaande tickets en diensten die verlopen | 0.0.1.0 |
| Tickets: aanmaken, reageren, sluiten, zoeken en filteren | 0.0.1.0 |
| Beheer: tickets van alle klanten beantwoorden | 0.0.1.0 |
| Het commando `stacklane` met setup, update en status | 0.0.1.0 |
| Gameservers via Pterodactyl: console, bestanden, back-ups | in ontwikkeling |
| Import uit Pterodactyl, WHMCS en Paymenter | in ontwikkeling |
| Wachtwoord herstellen per e-mail | in ontwikkeling |

Facturatie, VPS, gameservers, domeinen en webhosting staan op de [roadmap](/#roadmap).

## Hoe het in elkaar zit

Stacklane is één programma, geschreven in Rust. Het levert de API en het klantpanel, een React-app. De gegevens staan in PostgreSQL. Een reverse proxy zoals Caddy zet het panel op je domein.

```
browser ── https ──> Caddy ── http ──> stacklane ──> PostgreSQL
```

## Verder

- [Installeren](/docs/installation/install) op je eigen server
- De [demo](https://demo.stacklane.io/login) bekijken als klant of als beheerder
