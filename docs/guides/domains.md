# Domeinen met Openprovider

Klanten zoeken, registreren en verhuizen domeinen in Stacklane. Stacklane regelt de registratie bij Openprovider, maakt elk jaar een verlengfactuur, en verlengt het domein zodra die betaald is.

## Wat je nodig hebt

- Een resellersaccount bij [Openprovider](https://www.openprovider.com), met tegoed
- Bij voorkeur een aparte API-gebruiker in Openprovider, zodat je die los kunt intrekken

## Koppelen

1. Ga naar **Beheer → Koppelingen** en kies **Koppeling toevoegen**, met als platform **Openprovider**.
2. Vul de gebruikersnaam en het wachtwoord in. Het adres van de API mag leeg blijven.
3. Pas zo nodig de standaard nameservers aan. Nieuwe domeinen krijgen die nameservers.

Stacklane logt meteen in om de gegevens te testen en bewaart ze versleuteld met `SECRET_KEY`. Stacklane gebruikt de `/v1`-API van Openprovider; de oude `/v1beta` stopt op 30 juni 2027.

## Prijzen

Onder **Beheer → Domeinextensies** zet je per extensie, zoals `nl` of `com`, drie prijzen per jaar, inclusief btw:

- **Registreren**: het eerste jaar van een nieuw domein
- **Verlengen**: elk jaar daarna
- **Verhuizen**: het eerste jaar na een verhuizing

Klanten zien alleen extensies die op **te bestellen** staan. Premium domeinen kunnen ze niet zelf bestellen; ze krijgen de melding contact op te nemen.

## Hoe een klant bestelt

1. Onder **Domeinnamen → WHOIS-profielen** legt de klant de gegevens van de houder vast.
2. Onder **Domeinen → Domein registreren** zoekt hij een naam. Zonder extensie toont Stacklane meteen de beschikbaarheid bij je goedkoopste extensies.
3. Hij kiest een domein en een houder, en betaalt de factuur. Bij een verhuizing vult hij ook de autorisatiecode van zijn huidige provider in; Stacklane bewaart die versleuteld en gooit hem na gebruik weg.
4. Na betaling maakt Stacklane de houder aan bij Openprovider en registreert of verhuist het domein.

Een bestelling die na de vervaldatum nog 14 dagen open staat, vervalt vanzelf.

## Wat klanten zelf kunnen

- nameservers wijzigen
- de verhuisvergrendeling aan- en uitzetten
- de autorisatiecode opvragen om het domein elders onder te brengen
- de gegevens van de houder bijwerken; die gaan ook naar Openprovider. De naam van de houder ligt vast zodra er een domein op staat.

## Verlengen

Elk domein hoort bij een jaarabonnement tegen de verlengprijs. Stacklane maakt de verlengfactuur op tijd, incasseert die als de klant automatisch betaalt, en verlengt het domein bij Openprovider zodra de factuur betaald is. Lukt het verlengen niet, dan zie je de foutmelding bij de dienst onder **Beheer → Diensten**.
