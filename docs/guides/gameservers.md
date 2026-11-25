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

## Server delen met anderen

Een klant kan anderen toegang geven tot een server, bijvoorbeeld een medebeheerder van een Minecraft-community. Dat gaat via het tabblad **Gebruikers** op de serverpagina. Alleen de eigenaar ziet dat tabblad.

1. De ander heeft een eigen account nodig met een bevestigd e-mailadres.
2. De eigenaar vult dat e-mailadres in en vinkt de rechten aan.
3. De ander krijgt een melding en accepteert onder **Account → Gedeelde toegang**. Pas daarna werkt de toegang.

| Recht | Wat het geeft |
| --- | --- |
| Console | Live meekijken en commando's sturen. Daarmee kan iemand de server ook stoppen. |
| Starten en stoppen | De knoppen om te starten, herstarten en stoppen |
| Bestanden | Bestanden bekijken en wijzigen, ook configuratie met wachtwoorden |
| Back-ups | Back-ups maken en downloaden; een back-up bevat alle bestanden |
| Activiteit | Zien wie wat deed, zonder IP-adressen |

Een gedeelde server staat bij de ander onder **Gameservers → Servers**, met wie hem deelt. Een server kan met hoogstens tien gebruikers gedeeld worden. Wordt de server opgeschort of opgeheven, dan stopt de toegang voor iedereen.

De toegang loopt via Stacklane, niet via subusers in Pterodactyl. De ander heeft dus geen Pterodactyl-account nodig, en jij houdt alles in één panel.

## Plugins en mods

Op een Minecraft-server staat het tabblad **Plugins en mods**. Klanten zoeken daar in [Modrinth](https://modrinth.com) en installeren met één klik. Ze hebben het recht **Bestanden** nodig.

- Stacklane leest het servertype uit de naam van de egg, zoals Paper, Purpur, Fabric, Forge of NeoForge. Herkent het de egg niet, dan kiest de klant het zelf.
- De Minecraft-versie komt uit de startvariabele `MINECRAFT_VERSION` (of `MC_VERSION` of `VERSION`) als daar een vaste versie staat. Anders vult de klant die zelf in.
- Plugins komen in `/plugins`, mods in `/mods`. Een klant kan daar ook `.jar`-bestanden verwijderen.
- Na installeren of verwijderen moet de server opnieuw starten.

Stacklane haalt de downloadlink zelf op bij Modrinth en accepteert alleen bestanden van `cdn.modrinth.com`. Daarna laat Stacklane Pterodactyl het bestand ophalen. Een klant kan dus geen eigen URL laten downloaden.

Wat je nodig hebt:

- De server waarop Stacklane draait, moet `api.modrinth.com` kunnen bereiken.
- Wings moet `cdn.modrinth.com` kunnen bereiken. Wings blokkeert downloads van interne adressen standaard, en dat is goed zo.
- Wil je een andere spiegel gebruiken, zet dan `MODRINTH_URL` in `/etc/stacklane/stacklane.env`.

Modpacks installeren kan nog niet. Een modpack verandert meer dan losse bestanden, zoals de serverversie en de loader. Daarvoor kun je beter een egg met dat modpack gebruiken.

## Activiteit

Het tabblad **Activiteit** toont wie de server startte of stopte, de console opende, een bestand wijzigde of een back-up maakte. De eigenaar ziet ook vanaf welk IP-adres.

## Bestaande servers overnemen

Heb je al klanten in Pterodactyl, haal ze dan over met [`stacklane import pterodactyl`](/docs/guides/import).
