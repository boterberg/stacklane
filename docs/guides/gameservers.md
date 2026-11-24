# Gameservers met Pterodactyl

Stacklane maakt gameservers aan in je bestaande Pterodactyl-panel. Je klanten beheren ze daarna in Stacklane zelf: ze zien de console, starten en stoppen hun server, bewerken bestanden en maken back-ups. Ze hoeven niet in te loggen op Pterodactyl.

## Wat je nodig hebt

- Pterodactyl Panel 1.11 of nieuwer, met minstens één node waarop Wings draait
- Een **application API-sleutel** met lees- en schrijfrechten op gebruikers, servers, nests, eggs, locaties, nodes en allocaties
- Een **client API-sleutel** van een beheerdersaccount

De console in de browser praat rechtstreeks met Wings. Zet daarom het adres van je Stacklane-panel in `allowed_origins` van de Wings-configuratie, en laat Wings via HTTPS bereikbaar zijn als je panel ook HTTPS gebruikt.

## Koppelen

1. Ga naar **Beheer → Koppelingen** en kies **Koppeling toevoegen**.
2. Vul het adres van je Pterodactyl-panel en de twee sleutels in.
3. Stacklane test de verbinding meteen. De sleutels worden versleuteld opgeslagen met `SECRET_KEY`.

## Een product maken

Onder **Beheer → Producten** maak je een pakket, bijvoorbeeld *Minecraft 4 GB*. Je kiest:

- de koppeling, en daarna de locatie, het nest en de egg; die haalt Stacklane live uit Pterodactyl
- geheugen, opslag en processor
- hoeveel databases en back-ups de klant mag maken

## Een server leveren

Onder **Beheer → Diensten** kies je **Dienst aanmaken**, met de klant, het product en een naam. Stacklane maakt de gebruiker in Pterodactyl aan als die nog niet bestaat, en daarna de server. Na de installatie ziet de klant de server onder **Gameservers → Servers**.

Vanuit hetzelfde overzicht schort je een server op, heractiveer je hem of hef je hem op. Mislukt het aanmaken, dan zie je de foutmelding van Pterodactyl en kun je het opnieuw proberen.

## Bestaande servers overnemen

Heb je al klanten in Pterodactyl, haal ze dan over met [`stacklane import pterodactyl`](/docs/guides/import).
