# Importeren

Met `stacklane import` neem je klanten en diensten over uit een ander systeem. Een import kun je veilig vaker draaien: wat al in Stacklane staat, slaat hij over.

## Uit Pterodactyl

Koppel je panel eerst onder **Beheer → Koppelingen**. Kijk daarna wat de import zou doen:

```sh
sudo stacklane import pterodactyl --dry-run
```

Klopt dat, draai hem dan echt:

```sh
sudo stacklane import pterodactyl
```

Heb je meer dan één Pterodactyl-koppeling, kies er dan een met `--integration <naam>`.

Wat er gebeurt:

- Elke gebruiker met een server wordt een klant in Stacklane. Bestaat er al een account met hetzelfde e-mailadres, dan gebruikt de import dat, maar alleen als de klant dat adres heeft bevestigd of het account uit een eerdere import komt.
- Elke server wordt een gameserver bij die klant, meteen gekoppeld. Opgeschorte servers blijven opgeschort.
- Beheerders zonder eigen servers slaat de import over.

Nieuwe klanten krijgen geen wachtwoord. Ze stellen er zelf een in met **Wachtwoord vergeten**; zorg dus dat [`SMTP_URL`](/docs/installation/configuration) ingesteld is.

## Uit WHMCS

De import leest de WHMCS-database rechtstreeks, alleen lezend. Geef een MySQL-gebruiker op; het wachtwoord vraagt het commando apart, zodat het niet in je shellgeschiedenis komt:

```sh
sudo stacklane import whmcs --database mysql://whmcs@db.jouwbedrijf.nl/whmcs --dry-run
sudo stacklane import whmcs --database mysql://whmcs@db.jouwbedrijf.nl/whmcs
```

Wat er meekomt:

| Uit WHMCS | In Stacklane |
| --- | --- |
| Klanten, met adres en bedrijfsnaam | Klanten. Hun wachtwoord blijft werken en wordt bij de eerste keer inloggen opnieuw versleuteld. |
| Producten en hun prijs | Producten, met een maandprijs. Verborgen producten worden inactief. |
| Diensten (`tblhosting`) | Abonnementen en diensten, met status, vervaldatum en interval |
| Facturen en factuurregels | Facturen, met hetzelfde nummer en dezelfde status |
| Tickets en reacties | Tickets met het hele gesprek. Afdelingen die nog niet bestaan, worden aangemaakt. |

Wat de import overslaat, staat aan het eind van de uitvoer: klanten zonder geldig e-mailadres, klanten van wie het adres al bij een onbevestigd account hoort, tickets van gasten zonder account en losse betalingen. Diensten met een cyclus van twee of drie jaar komen binnen als jaarlijks.

De hele import is één transactie. Gaat er halverwege iets mis, dan verandert er niets.

## Uit Paymenter

Paymenter draait ook op MySQL of MariaDB, dus het werkt op dezelfde manier:

```sh
sudo stacklane import paymenter --database mysql://paymenter@db.jouwbedrijf.nl/paymenter --dry-run
sudo stacklane import paymenter --database mysql://paymenter@db.jouwbedrijf.nl/paymenter
```

Klanten met hun adresgegevens, producten, diensten, facturen en tickets komen mee. Wachtwoorden blijven werken. De naam die een klant aan een dienst gaf, wordt de naam van de dienst. Conceptfacturen slaat de import over.

Beheerders uit Paymenter worden geen klant. Maak hun account opnieuw aan met `sudo stacklane admin create`.

## Uit Blesta en HostBill

Deze imports komen in een volgende versie.
