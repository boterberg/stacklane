# Facturatie

Stacklane maakt zelf facturen voor verlengingen, laat klanten online betalen en zet diensten uit als een factuur te lang openstaat.

## Instellen

Ga naar **Beheer → Instellingen**.

**Bedrijf**: naam, adres, KVK- en btw-nummer, IBAN. Dit staat onderaan elke factuur en in de pdf.

**Facturatie**:

| Instelling | Standaard | Betekenis |
| --- | --- | --- |
| Btw-tarief | 21% | Prijzen zijn inclusief btw; de factuur rekent het btw-deel terug |
| Betaaltermijn | 14 dagen | Hoe lang een klant heeft om te betalen |
| Factuur maken | 14 dagen | Zoveel dagen voor de vervaldatum komt de verlengfactuur |
| Opschorten na | 7 dagen | Zoveel dagen na de vervaldatum gaan diensten uit als er nog een open factuur is |

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
