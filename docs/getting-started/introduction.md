# Introductie

Stacklane is het panel waar de klanten van je hostingbedrijf inloggen. Ze bestellen er gameservers, VPS'en en webhosting, betalen hun facturen en stellen vragen aan je supportteam. Je draait het zelf, op een server die je al hebt of een nieuwe.

::: warning Vroege versie
Stacklane is net begonnen. Kijk bij de [releases](/releases/) wat er in de huidige versie zit, en gebruik het nog niet voor betalende klanten.
:::

## Wat er nu in zit

| Onderdeel | Sinds |
| --- | --- |
| Accounts: registreren, inloggen, wachtwoord herstellen per e-mail | 1.0.0.0 |
| Dashboard met openstaande tickets en diensten die verlopen | 1.0.0.0 |
| Tickets: aanmaken, reageren, sluiten, zoeken en filteren | 1.0.0.0 |
| Beheer: tickets, facturen, diensten, producten en koppelingen | 1.0.0.0 |
| Gameservers via Pterodactyl: console, bestanden, back-ups | 1.0.0.0 |
| Facturatie: abonnementen, pdf-facturen, automatisch verlengen | 1.0.0.0 |
| Betalen met Mollie, Stripe of een overboeking | 1.0.0.0 |
| Tweestapsverificatie en API-tokens | 1.0.0.0 |
| Import uit Pterodactyl, WHMCS en Paymenter | 1.0.0.0 |
| VPS op Proxmox VE: bestellen, aan en uit, snapshots, firewall | 1.1.0.0 |
| Webhosting op DirectAdmin en Plesk, inloggen met één klik | 1.1.0.0 |
| Bestellen en betalen in het panel | 1.1.0.0 |
| Nederlands en Engels, licht en donker | 1.0.0.0 |
| Het commando `stacklane` met setup, update, status en import | 1.0.0.0 |

Domeinen, cPanel en VPS op OpenStack en VirtFusion staan op de [roadmap](/#roadmap).

## Hoe het in elkaar zit

Stacklane is één programma, geschreven in Rust. Het levert de API en het klantpanel, een React-app. De gegevens staan in PostgreSQL. Een reverse proxy zoals Caddy zet het panel op je domein.

```
browser ── https ──> Caddy ── http ──> stacklane ──> PostgreSQL
```

## Verder

- [Installeren](/docs/installation/install) op je eigen server
- De [demo](https://demo.stacklane.io/login) bekijken als klant of als beheerder
