# Webhosting met cPanel, DirectAdmin of Plesk

Stacklane verkoopt webhosting op je eigen hostingservers. Na betaling maakt Stacklane het account aan, bij een achterstand schort het het account op, en bij opzeggen heft het het account op. Klanten loggen met één klik in op hun hostingpanel, zonder apart wachtwoord.

## Een server koppelen

Ga naar **Beheer → Koppelingen** en kies **Koppeling toevoegen**.

### cPanel & WHM

- **Adres**: het adres van WHM, zoals `https://server.jouwbedrijf.nl:2087`
- **WHM-gebruiker**: `root` of een reseller
- **API-token**: maak er een aan onder **Development → Manage API Tokens** in WHM

Klanten loggen in via een sessie die WHM aanmaakt (`create_user_session`).

### DirectAdmin

- **Adres**: zoals `https://server.jouwbedrijf.nl:2222`
- **Gebruikersnaam**: een beheerder of reseller
- **Inlogsleutel**: maak een login key aan en sta deze opdrachten toe: `CMD_API_ACCOUNT_USER`, `CMD_API_SELECT_USERS`, `CMD_API_PACKAGES_USER` en `CMD_API_LOGIN_KEYS`
- **IP-adres voor websites**: het gedeelde IP-adres waarop nieuwe accounts komen

Klanten loggen in met een eenmalige inloglink. Daarvoor moeten login keys aan staan voor gebruikers.

### Plesk

- **Adres**: zoals `https://server.jouwbedrijf.nl:8443`
- **API-sleutel**: maak er een aan met `plesk bin secret_key -c`
- **IP-adres voor websites**: het gedeelde IP-adres waarop nieuwe abonnementen komen

Stacklane maakt per klant een Plesk-klant aan met een abonnement op het gekozen service plan. Inloggen gaat via een sessie die aan het IP-adres van de klant vastzit. Zet Stacklane daarom achter een proxy die `X-Forwarded-For` meestuurt, zoals Caddy of nginx.

Stacklane test de gegevens meteen en bewaart ze versleuteld met `SECRET_KEY`.

## Pakketten

Onder **Beheer → Producten** maak je een product van de soort **Webhosting**. Kies de server en het pakket (in Plesk: het service plan). Schijfruimte, e-mailadressen en databases stel je in op de server zelf.

## Hoe een klant bestelt

1. Onder **Webhosting → Webhosting bestellen** kiest de klant een pakket en vult het domein van de website in.
2. Na betaling maakt Stacklane het account aan. De gebruikersnaam volgt uit het domein, zoals `bakker5b` voor `bakkerijdejong.nl`. Het wachtwoord is willekeurig; de klant heeft het niet nodig.
3. Op de pagina van het pakket staat **Inloggen op cPanel** (of DirectAdmin, of Plesk).

Het domein zelf regelt de klant apart, bij jou onder **Domeinen** of bij een andere provider.
