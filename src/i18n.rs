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
        "Vul de WHM-gebruiker en een API-token in." => "Enter the WHM user and an API token.",
        "Kies een koppeling met cPanel, DirectAdmin of Plesk." => {
            "Choose an integration with cPanel, DirectAdmin or Plesk."
        }
        "Kies een hostingpakket." => "Choose a hosting package.",
        "Dit webhostingpakket is nog niet klaar." => "This web hosting package is not ready yet.",
        "Vul de domeinnaam van je website in, zoals jouwbedrijf.nl." => {
            "Enter your website's domain name, such as yourcompany.com."
        }
        "Vul het token-ID (gebruiker@realm!naam) en het geheim in." => {
            "Enter the token ID (user@realm!name) and the secret."
        }
        "Vul de node, de gateway, een geldige reeks IP-adressen en een VLAN-reeks tussen 1 en 4094 in." => {
            "Enter the node, the gateway, a valid range of IP addresses and a VLAN range between 1 and 4094."
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
        "Op dit platform kun je alleen verkeer toestaan." => {
            "On this platform you can only allow traffic."
        }
        "Vul het ID en het geheim van de application credential in." => {
            "Enter the ID and secret of the application credential."
        }
        "Vul het netwerk en minstens één image in." => {
            "Enter the network and at least one image."
        }
        "Op dit platform geef je per regel één poort of reeks op." => {
            "On this platform each rule takes one port or range."
        }
        "Vul een API-token van VirtFusion in." => "Enter a VirtFusion API token.",
        "Vul de hypervisorgroep en minstens één besturingssysteem in." => {
            "Enter the hypervisor group and at least one operating system."
        }
        "Kies een pakket van VirtFusion." => "Choose a VirtFusion package.",
        "Vul een voornaam in." => "Enter a first name.",
        "Vul een achternaam in." => "Enter a last name.",
        "Vul een straat in." => "Enter a street.",
        "Vul een huisnummer in." => "Enter a house number.",
        "Vul een postcode in." => "Enter a postcode.",
        "Vul een plaats in." => "Enter a city.",
        "Vul een telefoonnummer in met landcode, zoals +31 612345678." => {
            "Enter a phone number with country code, such as +31 612345678."
        }
        "De naam van de houder wijzig je met een verhuizing naar een nieuw profiel." => {
            "To change the holder's name, move the domain to a new profile."
        }
        "Dit profiel hoort nog bij een domein." => "This profile still belongs to a domain.",
        "Vul de gebruikersnaam en het wachtwoord van Openprovider in." => {
            "Enter the Openprovider username and password."
        }
        "Vul minstens twee geldige nameservers in." => "Enter at least two valid name servers.",
        "Vul een extensie in, zoals nl of co.uk." => "Enter an extension, such as nl or co.uk.",
        "Vul een prijs van 0 of hoger in." => "Enter a price of 0 or more.",
        "Kies een koppeling met een registrar." => "Choose an integration with a registrar.",
        "Deze extensie staat er al." => "This extension already exists.",
        "Vul een domeinnaam in, zoals jouwbedrijf of jouwbedrijf.nl." => {
            "Enter a domain name, such as yourcompany or yourcompany.nl."
        }
        "Vul een geldige domeinnaam in." => "Enter a valid domain name.",
        "Deze extensie kun je niet bestellen." => "This extension cannot be ordered.",
        "Kies een WHOIS-profiel." => "Choose a WHOIS profile.",
        "Vul de autorisatiecode van je huidige provider in." => {
            "Enter the authorisation code from your current provider."
        }
        "Dit is een premium domein. Neem contact met ons op voor de prijs." => {
            "This is a premium domain. Contact us for the price."
        }
        "Dit domein is nog vrij. Registreer het in plaats van het te verhuizen." => {
            "This domain is still available. Register it instead of transferring it."
        }
        "Dit domein is al bezet." => "This domain is already taken.",
        "Dit domein is al besteld." => "This domain has already been ordered.",
        "Dit domein is nog niet klaar." => "This domain is not ready yet.",
        "Vul twee tot zes geldige nameservers in." => "Enter two to six valid name servers.",
        "Kies een flavor." => "Choose a flavor.",
        "Kies TCP, UDP of ICMP." => "Choose TCP, UDP or ICMP.",
        "Vul een IPv4-adres of -reeks in, zoals 203.0.113.4 of 10.0.0.0/8." => {
            "Enter an IPv4 address or range, such as 203.0.113.4 or 10.0.0.0/8."
        }
        "Kies een kwartaal van 1 tot en met 4." => "Choose a quarter from 1 to 4.",
        "Geef de firewall een naam van hoogstens 60 tekens." => {
            "Give the firewall a name of at most 60 characters."
        }
        "Een firewall heeft hoogstens 50 regels." => "A firewall has at most 50 rules.",
        "Deze server kan geen firewall van Stacklane gebruiken." => {
            "This server cannot use a Stacklane firewall."
        }
        "Kies een van je eigen servers." => "Choose one of your own servers.",
        "Kies een grootte van minimaal 10 GB en maximaal 1 TB." => {
            "Choose a size of at least 10 GB and at most 1 TB."
        }
        "Kies een server." => "Choose a server.",
        "Kies een van je actieve servers." => "Choose one of your active servers.",
        "Dit volume kan niet bij deze server. Kies een server op hetzelfde platform." => {
            "This volume cannot go with this server. Choose a server on the same platform."
        }
        "Je hebt al 10 volumes. Zeg er eerst een op." => {
            "You already have 10 volumes. Cancel one first."
        }
        "Dit volume is nog niet klaar." => "This volume is not ready yet.",
        "Dit platform kan geen volumes beheren." => "This platform cannot manage volumes.",
        "Dit volume zit niet aan een server vast." => "This volume is not attached to a server.",
        "Koppel het volume eerst los van de huidige server." => {
            "Detach the volume from its current server first."
        }
        "Kies een koppeling met Proxmox VE of OpenStack." => {
            "Choose a Proxmox VE or OpenStack integration."
        }
        "Dit platform kan geen ISO's gebruiken." => "This platform cannot use ISOs.",
        "Geef de ISO een naam van hoogstens 60 tekens." => {
            "Give the ISO a name of at most 60 characters."
        }
        "Kies BIOS of UEFI." => "Choose BIOS or UEFI.",
        "Je hebt al 2 ISO's. Verwijder er eerst een." => {
            "You already have 2 ISOs. Delete one first."
        }
        "Je hebt minimaal één actieve VPS nodig voordat je ISO's kunt importeren." => {
            "You need at least one active VPS before you can import ISOs."
        }
        "Koppel de ISO eerst los van de server." => "Detach the ISO from the server first.",
        "Wacht tot de ISO klaar is met importeren." => "Wait until the ISO has finished importing.",
        "Koppel eerst de huidige ISO los." => "Detach the current ISO first.",
        "Kies minimaal één recht." => "Choose at least one permission.",
        "Je kunt alleen rechten geven die je zelf hebt." => {
            "You can only grant permissions you have yourself."
        }
        "Er is geen account met dit e-mailadres. Vraag de persoon eerst een account aan te maken." => {
            "There is no account with this email address. Ask the person to create an account first."
        }
        "Dit is de eigenaar van deze omgeving." => "This is the owner of this environment.",
        "Je kunt hoogstens 25 gebruikers toegang geven." => {
            "You can give access to at most 25 users."
        }
        "Deze gebruiker heeft al toegang." => "This user already has access.",
        "Vul een e-mailadres in." => "Enter an email address.",
        "Kies een ISO die klaar is." => "Choose an ISO that is ready.",
        "Kies het land van je bedrijf." => "Choose the country of your business.",
        "Vul een btw-nummer uit de EU in, zoals NL123456789B01." => {
            "Enter an EU VAT number, such as NL123456789B01."
        }
        "Dit btw-nummer is niet geldig volgens VIES." => "VIES says this VAT number is not valid.",
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
        "Gebruik hoogstens 100 tekens." => "Use at most 100 characters.",
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
        "Deze gebruiker heeft rechten die jij niet hebt. Vraag de eigenaar om dit aan te passen." => {
            "This user has rights you do not have. Ask the owner to change this."
        }
        "Deze domeinnaam is al in gebruik bij een ander pakket." => {
            "This domain name is already used by another package."
        }
        "Hier loopt al een actie. Probeer het over een paar seconden opnieuw." => {
            "Something is already running here. Try again in a few seconds."
        }
        "Een server kan samen hoogstens 100 poorten of reeksen uit firewalls krijgen. Koppel minder firewalls aan deze server." => {
            "A server can get at most 100 ports or ranges from firewalls in total. Attach fewer firewalls to this server."
        }
        "Een gekoppelde server is nu niet actief, dus de regels kunnen er niet af. Koppel die server eerst los of wacht tot hij weer actief is." => {
            "An attached server is not active right now, so its rules cannot be removed. Detach that server first or wait until it is active again."
        }
        "Kies waar je de floating IP wilt hebben." => "Choose where you want the floating IP.",
        "Je hebt al 10 floating IPs. Zeg er eerst een op." => {
            "You already have 10 floating IPs. Cancel one first."
        }
        "Kies een aantal. Je kunt in totaal 10 floating IPs hebben." => {
            "Choose a number. You can have 10 floating IPs in total."
        }
        "Deze floating IP is nog niet klaar." => "This floating IP is not ready yet.",
        "Dit platform kan geen floating IPs beheren." => {
            "This platform cannot manage floating IPs."
        }
        "Deze floating IP is niet aan een server gekoppeld." => {
            "This floating IP is not attached to a server."
        }
        "Deze floating IP kan niet naar deze server. Kies een server op hetzelfde platform." => {
            "This floating IP cannot go to this server. Choose a server on the same platform."
        }
        "Op dit moment kun je gameserver-, VPS-, volume-, floating IP- en webhostingproducten aanmaken." => {
            "Right now you can create game server, VPS, volume, floating IP and web hosting products."
        }
        "Kies een koppeling met Proxmox VE, of OpenStack met een extern netwerk voor floating IPs." => {
            "Choose a Proxmox VE connection, or OpenStack with an external network for floating IPs."
        }
        "Gebruik hoogstens 60 letters, cijfers, spaties, punten, streepjes of liggende streepjes." => {
            "Use at most 60 letters, digits, spaces, dots, hyphens or underscores."
        }
        "Op dit platform kun je geen netwerken maken." => {
            "You cannot create networks on this platform."
        }
        "Kies een privé IP-bereik, zoals 10.1.0.0/24, met een netmasker van /16 tot /28." => {
            "Choose a private IP range, such as 10.1.0.0/24, with a netmask from /16 to /28."
        }
        "Je hebt al 10 netwerken. Verwijder er eerst een." => {
            "You already have 10 networks. Delete one first."
        }
        "Je hebt al een netwerk met deze naam." => "You already have a network with this name.",
        "Dit IP-bereik overlapt met een van je andere netwerken." => {
            "This IP range overlaps with one of your other networks."
        }
        "Op dit platform is geen ruimte meer voor nieuwe netwerken." => {
            "There is no room for new networks on this platform."
        }
        "Koppel eerst alle servers los van dit netwerk." => {
            "Detach all servers from this network first."
        }
        "Deze server staat op een ander platform dan het netwerk." => {
            "This server is on a different platform than the network."
        }
        "Deze server zit al in drie netwerken." => "This server is already in three networks.",
        "Het IP-bereik van dit netwerk klopt niet." => "The IP range of this network is not valid.",
        "Dit netwerk heeft geen vrije adressen meer." => "This network has no free addresses left.",
        "Herbouwen is mislukt. Probeer het opnieuw of neem contact op met support." => {
            "Rebuilding failed. Try again or contact support."
        }
        "Stacklane werd herstart tijdens het herbouwen. Probeer het opnieuw." => {
            "Stacklane restarted while rebuilding. Please try again."
        }
        "Deze server kun je alleen in het panel van VirtFusion herbouwen." => {
            "You can only rebuild this server in the VirtFusion panel."
        }
        "Deze server wordt al herbouwd." => "This server is already being rebuilt.",
        "Deze server wordt herbouwd. Wacht tot dat klaar is." => {
            "This server is being rebuilt. Wait until it is done."
        }
        "Dit platform heeft geen rescue-modus." => "This platform has no rescue mode.",
        "Deze server staat al in rescue-modus." => "This server is already in rescue mode.",
        "Koppel eerst de ISO los voordat je rescue-modus inschakelt." => {
            "Detach the ISO before you turn on rescue mode."
        }
        "Zet eerst de rescue-modus uit." => "Turn off rescue mode first.",
        "Dit IP-adres hoort niet bij deze server." => {
            "This IP address does not belong to this server."
        }
        "Vul een volledige hostnaam in, zoals mail.jouwdomein.nl." => {
            "Enter a full hostname, such as mail.yourdomain.com."
        }
        "Laat deze hostnaam eerst met een A-record naar dit IP-adres wijzen." => {
            "Point this hostname to this IP address with an A record first."
        }
        "Voor dit IP-adres kun je hier geen reverse DNS instellen." => {
            "You cannot set reverse DNS for this IP address here."
        }
        "Vul de server-ID van PowerDNS in, meestal localhost." => {
            "Enter the PowerDNS server ID, usually localhost."
        }
        "In de demo kun je geen eigen account aanmaken. Log in als klant of beheerder." => {
            "You cannot create your own account in the demo. Sign in as a customer or as an administrator."
        }
        "Bestel floating IPs onder VPS, Floating IPs." => {
            "Order floating IPs under VPS, Floating IPs."
        }
        "Bevestig eerst je eigen e-mailadres voordat je iemand toegang geeft." => {
            "Confirm your own email address before you give someone access."
        }
        "Deze persoon heeft het e-mailadres nog niet bevestigd. Vraag dat eerst te doen." => {
            "This person has not confirmed their email address yet. Ask them to do that first."
        }
        "Deze server wordt nu herbouwd. Probeer het daarna opnieuw." => {
            "This server is being rebuilt right now. Try again afterwards."
        }
        "Er hangt een geblokkeerd volume of IP-adres aan deze server. Terugzetten kan pas als dat weer actief is." => {
            "A blocked volume or IP address belongs to this server. You can restore a snapshot once it is active again."
        }
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
