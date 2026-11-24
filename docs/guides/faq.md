# Veelgestelde vragen

## Kan ik het al gebruiken voor mijn klanten?

Nog niet. Facturatie en het beheren van servers en domeinen ontbreken nog. Die komen eerst.

## Waar staat de demo?

Op [demo.stacklane.io](https://demo.stacklane.io/login). Je logt in met één klik als klant of als beheerder. Elk heel uur zetten we alles terug.

## Welke database heb ik nodig?

PostgreSQL, versie 15 of nieuwer. MySQL en MariaDB werken niet.

## Is er een Docker-image?

Nee. Stacklane is gemaakt om als Debian-pakket met systemd te draaien, en daar richten we ons op.

## Hoe werk ik bij?

Met `sudo stacklane update`. Het maakt eerst een back-up van je database. Meer bij [Bijwerken](/docs/installation/updating).

## Mag ik het aanpassen?

Ja. Stacklane valt onder de MIT-licentie. De code staat op [GitHub](https://github.com/boterberg/stacklane); daar meld je ook fouten.
