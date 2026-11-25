# API

Alles wat je in het panel ziet, komt uit dezelfde JSON-API. Met een persoonlijk token gebruik je die API ook vanuit je eigen scripts.

## Een token maken

Ga naar **Beveiliging → API-tokens** (via je naam linksonder) en klik op **Token maken**. Geef het token een naam, kies hoe lang het geldig is en vink aan wat het mag:

| Onderdeel | Bekijken | Beheren |
| --- | --- | --- |
| Account en meldingen | `GET /api/me`, `/api/dashboard`, `/api/notifications` | thema en taal wijzigen via `PATCH /api/me/settings` |
| Gameservers | lijst, details, bestanden, back-ups | aan/uit zetten, bestanden schrijven, console |
| Facturering | abonnementen, facturen, pdf's | betalen, opzeggen, betaalmethode kiezen |
| Ondersteuning | tickets en afdelingen | tickets maken, reageren, sluiten |
| Beheer | alleen voor beheerders: alles onder `/api/admin` lezen | alles onder `/api/admin` |

Beheren geeft ook leesrechten. Het token zie je maar één keer; Stacklane bewaart alleen een hash.

Een token kan nooit tokens maken of verwijderen, je wachtwoord wijzigen of andere sessies uitloggen. Daarvoor moet je echt ingelogd zijn.

Alles wat een wachtwoord of een directe ingang tot een server geeft, gaat ook niet met een token: het rootwachtwoord, de rescue-modus, een ISO koppelen, de login voor het VirtFusion- of hostingpanel en de autorisatiecode van een domein.

## Aanroepen

Stuur het token mee in de `Authorization`-header:

```sh
curl https://panel.jouwbedrijf.nl/api/gameservers \
  -H "Authorization: Bearer stl_..."
```

| Status | Betekenis |
| --- | --- |
| `401` | Token onbekend, verwijderd of verlopen |
| `403` | Het token mag dit niet; vink het onderdeel aan of kies Beheren |
| `422` | Er klopt een veld niet; de fout staat per veld in `errors` |

Wil je een Nederlandse of Engelse foutmelding, stuur dan `Accept-Language: nl` of `en` mee.

## Veelgebruikte routes

| Methode | Route | Wat |
| --- | --- | --- |
| `GET` | `/api/gameservers` | Je gameservers |
| `POST` | `/api/gameservers/{id}/power` | `{"signal": "start"}`, `stop`, `restart` of `kill` |
| `GET` | `/api/billing/invoices?status=open` | Openstaande facturen |
| `GET` | `/api/billing/invoices/{id}/pdf` | Factuur als pdf |
| `GET` | `/api/billing/subscriptions` | Je abonnementen |
| `GET` | `/api/support/tickets` | Je tickets |
| `POST` | `/api/support/tickets` | Ticket maken |

Facturen, abonnementen en tickets zijn gepagineerd met `?page=` en `?per_page=`; het antwoord heeft `data` en `meta` met het totaal.
