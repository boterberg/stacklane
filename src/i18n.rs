use axum::extract::Request;
use axum::http::header::ACCEPT_LANGUAGE;
use axum::middleware::Next;
use axum::response::Response;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    Nl,
    En,
}

impl Locale {
    pub fn parse(value: &str) -> Self {
        if value.trim().to_lowercase().starts_with("en") {
            Self::En
        } else {
            Self::Nl
        }
    }
}

tokio::task_local! {
    static LOCALE: Locale;
}

pub fn current() -> Locale {
    LOCALE.try_with(|locale| *locale).unwrap_or(Locale::Nl)
}

pub async fn middleware(request: Request, next: Next) -> Response {
    let locale = request
        .headers()
        .get(ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .map(Locale::parse)
        .unwrap_or(Locale::Nl);
    LOCALE.scope(locale, next.run(request)).await
}

pub fn tr(message: &str) -> String {
    match current() {
        Locale::Nl => message.to_owned(),
        Locale::En => english(message)
            .map(str::to_owned)
            .unwrap_or_else(|| message.to_owned()),
    }
}

fn english(message: &str) -> Option<&'static str> {
    Some(match message {
        "Je bent niet ingelogd." => "You are not signed in.",
        "Je hebt geen toegang tot dit onderdeel." => "You do not have access to this page.",
        "Niet gevonden." => "Not found.",
        "Controleer de ingevulde gegevens." => "Please check the details you entered.",
        "Er ging iets mis." => "Something went wrong.",
        "Vul je naam in." => "Enter your name.",
        "Vul een geldig e-mailadres in." => "Enter a valid email address.",
        "Je wachtwoord moet minstens 10 tekens lang zijn." => {
            "Your password must be at least 10 characters long."
        }
        "Kies een land." => "Choose a country.",
        "Vul je bedrijfsnaam in." => "Enter your company name.",
        "Er bestaat al een account met dit e-mailadres." => {
            "An account with this email address already exists."
        }
        "Deze gegevens kloppen niet met onze administratie." => {
            "These details do not match our records."
        }
        "Deze link is verlopen of al gebruikt. Vraag een nieuwe aan." => {
            "This link has expired or was already used. Request a new one."
        }
        "Ongeldig pad." => "Invalid path.",
        "Deze server is opgeschort. Neem contact op met support." => {
            "This server is suspended. Please contact support."
        }
        "Deze server is nog niet klaar." => "This server is not ready yet.",
        "Onbekende actie." => "Unknown action.",
        "Dit bestand is te groot om hier te bewerken." => "This file is too large to edit here.",
        "Kies licht, donker of systeem." => "Choose light, dark or system.",
        "Deze taal wordt niet ondersteund." => "This language is not supported.",
        "Je bericht is te lang." => "Your message is too long.",
        "Schrijf een bericht." => "Write a message.",
        "Kies een afdeling." => "Choose a department.",
        "Vat je vraag samen in minstens 3 tekens." => {
            "Summarise your question in at least 3 characters."
        }
        "Gebruik maximaal 150 tekens." => "Use at most 150 characters.",
        "Deze afdeling bestaat niet." => "This department does not exist.",
        "Onbekende status." => "Unknown status.",
        "Kies een type." => "Choose a type.",
        "Deze koppeling komt in een volgende versie." => {
            "This integration is coming in a later version."
        }
        "Geef de koppeling een naam." => "Give the integration a name.",
        "Vul een adres in dat met https:// begint." => {
            "Enter an address that starts with https://."
        }
        "Vul de application- en de client-API-sleutel in." => {
            "Enter both the application and the client API key."
        }
        "Er draaien nog diensten op deze koppeling." => {
            "There are still services running on this integration."
        }
        "Geef het product een naam." => "Give the product a name.",
        "De prijs kan niet negatief zijn." => "The price cannot be negative.",
        "Op dit moment kun je alleen gameserverproducten aanmaken." => {
            "For now you can only create game server products."
        }
        "Kies op welke koppeling de servers komen." => {
            "Choose the integration the servers will run on."
        }
        "Kies een locatie en een egg, met minstens 128 MB geheugen en 256 MB schijf." => {
            "Choose a location and an egg, with at least 128 MB of memory and 256 MB of disk."
        }
        "Er bestaat al een product met deze naam." => "A product with this name already exists.",
        "Er is geen klant met dit e-mailadres." => "There is no customer with this email address.",
        "Kies een product." => "Choose a product.",
        "Geef de dienst een naam van hoogstens 60 tekens." => {
            "Give the service a name of at most 60 characters."
        }
        "Alleen een mislukte dienst kun je opnieuw aanmaken." => {
            "Only a failed service can be created again."
        }
        "Deze actie kan niet in de huidige status." => {
            "This action is not possible in the current status."
        }
        "Bijwerken kan niet in de demo." => "Updating is not available in the demo.",
        "Bijwerken vanuit het panel is op deze server niet ingesteld." => {
            "Updating from the panel is not set up on this server."
        }
        "Er loopt al een update." => "An update is already running.",
        "Er is geen nieuwere versie." => "There is no newer version.",
        "Deze betaalmethode is niet beschikbaar." => "This payment method is not available.",
        "In de demo kun je niet echt betalen." => "You cannot really pay in the demo.",
        "Vul eerst een API-sleutel van Mollie in." => "Enter a Mollie API key first.",
        "Vul eerst de geheime sleutel en het webhookgeheim van Stripe in." => {
            "Enter the Stripe secret key and webhook secret first."
        }
        "Dit abonnement is al opgezegd." => "This subscription has already been cancelled.",
        "Dit abonnement kun je niet meer hervatten." => {
            "This subscription can no longer be resumed."
        }
        "Kies een opgeslagen betaalmethode." => "Choose a saved payment method.",
        "Deze factuur staat niet meer open." => "This invoice is no longer unpaid.",
        "Geef de token een naam van hoogstens 80 tekens." => {
            "Give the token a name of at most 80 characters."
        }
        "Kies minstens één bevoegdheid." => "Choose at least one permission.",
        "Deze bevoegdheid bestaat niet." => "This permission does not exist.",
        "Kies een geldigheid tussen 1 en 365 dagen." => "Choose a lifetime between 1 and 365 days.",
        "Je hebt al 25 tokens. Verwijder er eerst een." => {
            "You already have 25 tokens. Delete one first."
        }
        "Dit wachtwoord klopt niet." => "This password is not correct.",
        "In de demo kun je het wachtwoord niet wijzigen." => {
            "You cannot change the password in the demo."
        }
        "In de demo kun je 2FA niet aanzetten." => "You cannot turn on 2FA in the demo.",
        "2FA staat al aan." => "2FA is already on.",
        "Begin opnieuw met instellen." => "Start the setup again.",
        "Deze code klopt niet. Kijk of de tijd op je telefoon goed staat." => {
            "This code is not correct. Check that the time on your phone is right."
        }
        "Zet eerst 2FA aan." => "Turn on 2FA first.",
        "Vul de code uit je authenticator-app in." => "Enter the code from your authenticator app.",
        "Deze code klopt niet." => "This code is not correct.",
        "Kies een tijdzone uit de lijst." => "Choose a time zone from the list.",
        "Geef de sleutel een naam van hoogstens 60 tekens." => {
            "Give the key a name of at most 60 characters."
        }
        "Dit is geen geldige publieke SSH-sleutel." => "This is not a valid public SSH key.",
        "Deze sleutel heb je al toegevoegd." => "You already added this key.",
        "Vul de gebruikersnaam en een inlogsleutel van DirectAdmin in." => {
            "Enter the DirectAdmin username and a login key."
        }
        "Vul een API-sleutel van Plesk in." => "Enter a Plesk API key.",
        "Vul het IP-adres in waarop nieuwe websites komen." => {
            "Enter the IP address new websites are placed on."
        }
        "Op dit moment kun je gameserver-, VPS- en webhostingproducten aanmaken." => {
            "Right now you can create game server, VPS and web hosting products."
        }
        "Kies een koppeling met DirectAdmin of Plesk." => {
            "Choose an integration with DirectAdmin or Plesk."
        }
        "Kies een hostingpakket." => "Choose a hosting package.",
        "Dit webhostingpakket is nog niet klaar." => "This web hosting package is not ready yet.",
        "Vul de domeinnaam van je website in, zoals jouwbedrijf.nl." => {
            "Enter your website's domain name, such as yourcompany.com."
        }
        "Vul het token-ID (gebruiker@realm!naam) en het geheim in." => {
            "Enter the token ID (user@realm!name) and the secret."
        }
        "Vul de node, de gateway en een geldige reeks IP-adressen in." => {
            "Enter the node, the gateway and a valid range of IP addresses."
        }
        "In de demo kun je niets bestellen." => "You cannot order anything in the demo.",
        "Kies een pakket." => "Choose a plan.",
        "Kies hoe vaak je wilt betalen." => "Choose how often you want to pay.",
        "Geef de server een naam van hoogstens 60 tekens." => {
            "Give the server a name of at most 60 characters."
        }
        "Kies een besturingssysteem." => "Choose an operating system.",
        "Kies je eigen SSH-sleutels." => "Choose your own SSH keys.",
        "Deze server is nu niet beschikbaar." => "This server is not available right now.",
        "Je hebt al 3 snapshots. Verwijder er eerst een." => {
            "You already have 3 snapshots. Delete one first."
        }
        "Kies een richting en of het verkeer mag." => {
            "Choose a direction and whether traffic is allowed."
        }
        "Kies TCP, UDP of ICMP." => "Choose TCP, UDP or ICMP.",
        "Vul een IPv4-adres of -reeks in, zoals 203.0.113.4 of 10.0.0.0/8." => {
            "Enter an IPv4 address or range, such as 203.0.113.4 or 10.0.0.0/8."
        }
        "Vul je voornaam in." => "Enter your first name.",
        "In de demo kun je het e-mailadres niet wijzigen." => {
            "You cannot change the email address in the demo."
        }
        "Kies handmatig of automatisch." => "Choose manual or automatic.",
        "In de demo kun je je account niet verwijderen." => {
            "You cannot delete your account in the demo."
        }
        "Zeg eerst je abonnementen op en betaal je openstaande facturen." => {
            "Cancel your subscriptions and pay your open invoices first."
        }
        "Vul de naam van je bedrijf in." => "Enter the name of your company.",
        "Kies een btw-tarief tussen 0 en 100 procent." => {
            "Choose a VAT rate between 0 and 100 percent."
        }
        "Gebruik een aantal dagen tussen 0 en 365." => "Use a number of days between 0 and 365.",
        "We hebben net een e-mail gestuurd. Probeer het over een minuut opnieuw." => {
            "We just sent an email. Try again in a minute."
        }
        "Te veel foute codes. Probeer het over een paar minuten opnieuw." => {
            "Too many wrong codes. Try again in a few minutes."
        }
        "Te veel pogingen. Probeer het over een minuut opnieuw." => {
            "Too many attempts. Try again in a minute."
        }
        "Het is even te druk. Probeer het over een paar seconden opnieuw." => {
            "It is busy right now. Try again in a few seconds."
        }
        "Vul hoogstens 15 poorten of reeksen in, zoals 22, 80,443 of 8000:8100." => {
            "Enter at most 15 ports or ranges, such as 22, 80,443 or 8000:8100."
        }
        "Deze server heeft al 100 firewallregels. Verwijder er eerst een paar." => {
            "This server already has 100 firewall rules. Remove some first."
        }
        "Een firewall heeft samen hoogstens 100 poorten of reeksen." => {
            "A firewall has at most 100 ports or ranges in total."
        }
        "Je hebt al 20 firewalls. Verwijder er eerst een." => {
            "You already have 20 firewalls. Remove one first."
        }
        "Er ging iets mis bij het platform. Probeer het later opnieuw of neem contact op met support." => {
            "Something went wrong at the platform. Try again later or contact support."
        }
        "In de demo kun je dit niet wijzigen." => "You cannot change this in the demo.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn translates_inside_an_english_scope() {
        assert_eq!(tr("Niet gevonden."), "Niet gevonden.");
        let english = LOCALE
            .scope(Locale::En, async { tr("Niet gevonden.") })
            .await;
        assert_eq!(english, "Not found.");
        let unknown = LOCALE.scope(Locale::En, async { tr("Iets anders") }).await;
        assert_eq!(unknown, "Iets anders");
    }

    #[test]
    fn parses_accept_language() {
        assert_eq!(Locale::parse("en"), Locale::En);
        assert_eq!(Locale::parse("en-GB,en;q=0.9"), Locale::En);
        assert_eq!(Locale::parse("nl-NL,nl;q=0.9,en;q=0.8"), Locale::Nl);
    }
}
