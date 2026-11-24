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

- Elke gebruiker met een server wordt een klant in Stacklane. Bestaat er al een account met hetzelfde e-mailadres, dan gebruikt de import dat.
- Elke server wordt een gameserver bij die klant, meteen gekoppeld. Opgeschorte servers blijven opgeschort.
- Beheerders zonder eigen servers slaat de import over.

Nieuwe klanten krijgen geen wachtwoord. Ze stellen er zelf een in met **Wachtwoord vergeten**; zorg dus dat [`SMTP_URL`](/docs/installation/configuration) ingesteld is.

## Uit WHMCS, Paymenter, Blesta en HostBill

Deze imports komen in de volgende versies.
