# Facturatie

Stacklane maakt zelf facturen voor verlengingen, laat klanten online betalen en zet diensten uit als een factuur te lang openstaat.

## Instellen

Ga naar **Beheer → Instellingen**.

**Bedrijf**: naam, adres, KVK- en btw-nummer, IBAN. Dit staat onderaan elke factuur en in de pdf.

**Facturatie**:

| Instelling | Standaard | Betekenis |
| --- | --- | --- |
| Land van je bedrijf | Nederland | Bepaalt wie een binnenlandse klant is |
| Btw-tarief | 21% | Prijzen zijn inclusief btw; de factuur rekent het btw-deel terug |
| OSS-regeling | uit | Particulieren in andere EU-landen betalen het tarief van hun eigen land |
| Betaaltermijn | 14 dagen | Hoe lang een klant heeft om te betalen |
| Factuur maken | 14 dagen | Zoveel dagen voor de vervaldatum komt de verlengfactuur |
| Opschorten na | 7 dagen | Zoveel dagen na de vervaldatum gaan diensten uit als er nog een open factuur is |

## Btw voor klanten in het buitenland

Je prijzen zijn inclusief het btw-tarief van je eigen land. Op elke factuur kijkt Stacklane naar het land van de klant, en of die een gecontroleerd btw-nummer heeft:

| Klant | Btw op de factuur | Vermelding |
| --- | --- | --- |
| In je eigen land | Je eigen tarief | |
| Particulier in een ander EU-land, OSS uit | Je eigen tarief | |
| Particulier in een ander EU-land, OSS aan | Het tarief van het land van de klant | Btw tegen het tarief van dat land (OSS-regeling) |
| Bedrijf in een ander EU-land met een gecontroleerd btw-nummer | 0% | Btw verlegd, met het btw-nummer van de klant |
| Buiten de EU | 0% | Geen EU-btw in rekening gebracht |

Stacklane rekent je prijs eerst terug naar het bedrag zonder btw en telt daar het tarief van de klant bij op. Een pakket van € 12,10 kost een Duitse particulier met OSS dus € 11,90, en een bedrijf met verlegde btw € 10,00.

Zakelijke klanten vullen hun btw-nummer in bij hun profiel. Stacklane controleert het meteen bij VIES, de controledienst van de Europese Commissie. Is VIES even niet bereikbaar, dan probeert Stacklane het bij elke facturatierun opnieuw; tot die tijd krijgt de klant een gewone factuur.

Zet OSS aan zodra je over de EU-drempel van € 10.000 per jaar aan particulieren in andere EU-landen gaat, of eerder als je dat wilt. Je meldt je daarvoor zelf aan voor de OSS-regeling bij de Belastingdienst.

### Btw-overzicht

Onder **Beheer → Btw-overzicht** zie je per kwartaal de omzet en btw per regeling, land en tarief, op factuurdatum. Geannuleerde facturen tellen niet mee. De regels met OSS neem je over in je OSS-aangifte; met **CSV downloaden** krijg je hetzelfde overzicht als bestand.

De tarieven komen uit een vaste lijst met de standaardtarieven van alle EU-landen (stand 2026). Hosting valt overal onder het standaardtarief.

## Betaalmethoden

Je kunt er één of meer aanzetten onder **Betalingen**. Sleutels worden versleuteld opgeslagen met je `SECRET_KEY` en komen nooit meer terug in het panel.

### Mollie

Plak je API-sleutel uit het Mollie-dashboard (`live_...` of `test_...`). Stacklane controleert de sleutel bij het opslaan. De webhook hoef je niet in te stellen: Stacklane geeft hem bij elke betaling mee, naar `/api/webhooks/mollie`.

Klanten betalen met iDEAL, Bancontact of kaart. Vinken ze aan dat ze de betaalmethode willen bewaren, dan maakt Mollie een machtiging en schrijft Stacklane volgende verlengingen automatisch af.

### Stripe

Vul de geheime sleutel (`sk_live_...`) in en maak in het Stripe-dashboard een webhook naar:

```
https://panel.jouwbedrijf.nl/api/webhooks/stripe
```

met de events `checkout.session.completed`, `checkout.session.expired`, `payment_intent.succeeded` en `payment_intent.payment_failed`. Het webhookgeheim (`whsec_...`) vul je ook in Stacklane in; berichten zonder geldige handtekening worden geweigerd.

### Overboeking

Staat overboeking aan, dan ziet de klant bij het betalen je IBAN, het bedrag en het factuurnummer. Komt het geld binnen, open dan de factuur onder **Beheer → Alle facturen** en klik op **Op betaald zetten**.

::: tip
`PUBLIC_URL` moet het adres zijn waarop klanten het panel openen. Mollie en Stripe sturen de klant daarheen terug na het betalen, en de webhooks moeten het panel van buitenaf kunnen bereiken.
:::

## Verlengen

Elk uur kijkt Stacklane welke abonnementen binnenkort verlopen:

1. Er komt een factuur voor de volgende periode, met de prijzen die bij het abonnement horen.
2. Heeft het abonnement een opgeslagen betaalmethode, dan wordt het bedrag meteen afgeschreven. Anders krijgt de klant een melding en een e-mail.
3. Zodra de factuur betaald is, schuift de vervaldatum een periode op. Opgeschorte diensten gaan weer aan.
4. Staat de factuur na de vervaldatum plus de opschortingstermijn nog open, dan worden de diensten opgeschort.

Opgezegde abonnementen lopen door tot de vervaldatum en eindigen daarna. Tot die dag kan de klant de opzegging intrekken.

Wil je de ronde meteen draaien, bijvoorbeeld na het aanpassen van een vervaldatum:

```sh
sudo stacklane billing run
```

## Wat de klant ziet

Onder **Facturering** staan de abonnementen en facturen van de klant. Bij een open factuur staat de knop **Betalen**; elke factuur is als pdf te downloaden. Bij een abonnement kiest de klant tussen handmatig betalen en automatisch met een opgeslagen betaalmethode, en daar zegt de klant ook op.
