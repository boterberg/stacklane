# Team en rechten

Werk je met meer mensen, geef ze dan elk een eigen account. Zo zie je wie wat doet, en kan iemand van support niet per ongeluk aan je betaalinstellingen komen.

## Iemand toevoegen

1. Laat diegene zelf een account aanmaken via de registratiepagina. Zo kiest diegene zelf een wachtwoord, en hoef jij er geen te versturen.
2. Ga naar **Beheer → Team** en vul het e-mailadres van dat account in.
3. Kies **Medewerker** of **Beheerder**, vink de rechten aan en klik op **Toevoegen**.

Diegene ziet het beheer meteen, zonder opnieuw in te loggen.

## Rollen

Een **beheerder** mag alles. Alleen een beheerder kan instellingen, betaalmethoden, koppelingen, de maillog, updates en het team beheren.

Een **medewerker** mag alleen wat je aanvinkt:

| Recht | Wat het geeft |
| --- | --- |
| Support | Alle tickets beantwoorden en de kennisbank bijhouden |
| Facturatie | Facturen, betalingen markeren, btw-overzicht en statistieken |
| Klanten | Het klantenoverzicht, met de diensten en facturen per klant |
| Diensten en producten | Diensten aanmaken en beheren, producten en domeinextensies |

Het menu laat een medewerker alleen de pagina's zien waar diegene bij mag. De server controleert dat bij elk verzoek opnieuw, dus een link die je doorstuurt geeft geen extra toegang.

Meldingen over mislukte betalingen gaan naar iedereen met **Facturatie**. Meldingen over mislukte domeinverlengingen gaan naar iedereen met **Diensten en producten**.

## Rechten wijzigen of iemand weghalen

Klik bij een teamlid op **Rechten wijzigen**. De nieuwe rechten gelden bij het volgende verzoek.

**Uit team halen** maakt van het account weer een gewoon klantaccount. API-tokens met beheerrechten worden meteen ingetrokken.

Je kunt je eigen rechten niet wijzigen en jezelf niet weghalen. Zo blijft er altijd een beheerder over.

## Tweestapsverificatie

Bij teamleden zonder tweestapsverificatie staat **Geen 2FA**. Vraag ze die aan te zetten onder **Account → Beveiliging**. Een gestolen wachtwoord van een medewerker geeft anders toegang tot de gegevens van je klanten.
