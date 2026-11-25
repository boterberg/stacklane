# Huisstijl

Stacklane draait onder jouw naam. Onder **Beheer → Instellingen** zet je de huisstijl die je klanten zien.

## Naam

**Naam van het paneel** staat in de titel van het tabblad, boven het inlogscherm en in de zijbalk als je geen logo hebt.

## Thema

Je ziet elke wijziging meteen in je eigen scherm. Klanten zien het pas als je op **Opslaan** klikt.

**Kant-en-klare thema's** zetten in één klik een accentkleur en passende grijstinten. Daarna kun je alles nog aanpassen. **Standaard** zet alles terug.

**Accentkleur** kleurt de hoofdknoppen, het actieve menu-item en tabblad, voortgangsbalken, vinkjes en de grafieken, bijvoorbeeld `#e11d48`. Stacklane kiest zelf witte of donkere tekst op de knop, zodat die leesbaar blijft. Laat het veld leeg voor zwart-wit.

**Accentkleur in donkere modus** is optioneel. Een donkere kleur zoals `#0369a1` valt weg op een donkere achtergrond; kies daar een lichtere tint, zoals `#38bdf8`. Laat je het leeg, dan gebruikt Stacklane de gewone accentkleur.

**Achtergrond en tekst** bepaalt de grijstinten van het hele panel. Kies grijs, leisteen (blauwig), koel grijs, neutraal, warm grijs, of **Getint met accent**. Die laatste mengt een vleugje van je accentkleur in alle grijstinten.

**Hoeken** maakt knoppen, velden en kaarten strak, standaard of rond.

## Eigen CSS

Wil je verder gaan, zoals een eigen lettertype of een gekleurde zijbalk, zet dan je eigen CSS onder **Eigen CSS**. Die laadt na de standaardstijl, op elke pagina en ook op het inlogscherm. Je kunt hoogstens 64 kB kwijt. Laat het veld leeg om het uit te zetten.

Handige variabelen:

| Variabele | Wat het is |
| --- | --- |
| `--accent` | De accentkleur, al aangepast aan licht of donker |
| `--accent-contrast` | Leesbare tekstkleur op de accentkleur |
| `--color-zinc-50` t/m `--color-zinc-950` | De grijstinten van het panel |
| `--radius-lg` | De afronding van de meeste kaarten en velden |

```css
html.dark body {
  background: #0b1220;
}

.accent-fill {
  border-radius: 9999px;
}
```

De CSS is van jou: Stacklane controleert hem niet. Externe stylesheets en lettertypes laadt de browser niet, door het beveiligingsbeleid van het panel. Zet een lettertype dus als `data:`-URL in je CSS. Klopt er iets niet meer, maak het veld dan leeg en sla op.

## Logo en favicon

Upload een PNG, JPG of WebP van hoogstens 512 kB. Wijzigingen staan direct live.

| Bestand | Tip |
| --- | --- |
| Logo | Breed, zoals 600 × 120 pixels, voor een lichte achtergrond |
| Logo voor donkere modus | Optioneel; zonder dit bestand gebruiken we het gewone logo |
| Favicon | Vierkant, zoals 64 × 64 pixels; een ICO mag ook |

SVG-bestanden kun je niet uploaden. Een SVG kan scripts bevatten, en een logo moet nooit code in het panel kunnen draaien.
