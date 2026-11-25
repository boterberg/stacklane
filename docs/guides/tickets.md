# Tickets

Met tickets stellen klanten vragen aan je supportteam. Elk ticket hoort bij een afdeling en heeft een gespreksgeschiedenis.

## Een ticket maken

Onder **Ondersteuning → Tickets** klikt de klant op **Ticket maken** en vult in:

- **Afdeling**: VPS, Webhosting, Gameservers, Algemeen, Sales of Domeinen
- **Onderwerp**: een korte samenvatting, 3 tot 150 tekens
- **Bericht**: met opmaak, lijsten, citaten, code en links

De server haalt alles uit het bericht wat niet veilig is, zoals scripts en `javascript:`-links.

## Statussen

| Status | Betekenis |
| --- | --- |
| Open | Wacht op een reactie van support |
| Beantwoord | Support heeft gereageerd |
| Wachten op antwoord | Support wacht op informatie van de klant |
| In uitvoering | Support werkt eraan |
| Gesloten | Afgerond |

Reageert de klant op een gesloten ticket, dan gaat het weer open.

## Zoeken en filteren

De lijst toont standaard alle tickets behalve gesloten tickets. De klant zoekt op onderwerp of ticketnummer, filtert op status en kiest tussen kaarten en een tabel.

## Voor beheerders

Beheerders zien onder **Beheer → Alle tickets** de tickets van alle klanten, met de naam van de klant erbij. Ze zoeken ook op e-mailadres. Op een ticket reageren ze als support: het ticket gaat dan naar **Beantwoord**. De status zetten ze zelf met de keuzelijst bovenaan het ticket.

### Mail naar de klant

Reageert support op een ticket, dan krijgt de klant een melding in het panel en een e-mail met het antwoord erin. Stel daarvoor [SMTP](../installation/configuration) in.

Onder **Beheer → Maillog** zie je elke e-mail die Stacklane de laatste 90 dagen verstuurde, en of dat lukte. De inhoud bewaren we niet, want daar staan soms inloglinks in.

### Tickets automatisch sluiten

Onder **Beheer → Instellingen → Support** kies je na hoeveel dagen zonder reactie een beantwoord ticket dichtgaat. De klant krijgt dan een melding en kan het ticket heropenen door te reageren. Met 0 blijven tickets open.

## Helpcentrum

Onder **Beheer → Kennisbank** schrijf je artikelen voor het helpcentrum. Geef elk artikel een categorie, zoals VPS of Facturering. Alleen gepubliceerde artikelen zijn zichtbaar. Klanten vinden ze onder **Ondersteuning → Helpcentrum** en kunnen erin zoeken. Onder elk artikel staat een knop om toch een ticket te openen.

Heb je al een helpcentrum op een eigen website, zet dan `VITE_HELP_CENTER_URL` bij het bouwen van de frontend; de link gaat dan daarheen.
