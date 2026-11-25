# VPS met Proxmox VE, OpenStack of VirtFusion

Stacklane maakt virtuele servers aan op je eigen Proxmox VE-cluster, in een OpenStack-cloud of via VirtFusion. Klanten bestellen een server in het panel, betalen de factuur, en binnen een paar minuten draait hij. Daarna zetten ze hem zelf aan en uit, maken ze snapshots en stellen ze de firewall in.

## Wat je nodig hebt

- Proxmox VE 8 of 9
- Een **API-token** met rechten op de VM's en de opslag, bijvoorbeeld de rol `PVEVMAdmin` plus `Datastore.AllocateSpace` op je opslag
- Eén of meer **cloud-init-templates**, bijvoorbeeld van de officiële cloud-images van Debian of Ubuntu
- Een reeks vrije IPv4-adressen op de bridge waar de servers aan komen
- De **firewall van het datacenter** aan (*Datacenter → Firewall → Opties*). Zonder die schakelaar werken de firewall en het IP-filter per server niet; de koppelingstest waarschuwt als hij uit staat

Een template maak je zo, hier met Debian 13 als VM 9000:

```sh
wget https://cloud.debian.org/images/cloud/trixie/latest/debian-13-genericcloud-amd64.qcow2
qm create 9000 --name debian-13 --memory 1024 --net0 virtio,bridge=vmbr0 --scsihw virtio-scsi-pci
qm importdisk 9000 debian-13-genericcloud-amd64.qcow2 local
qm set 9000 --scsi0 local:9000/vm-9000-disk-0.raw --ide2 local:cloudinit --boot order=scsi0 --serial0 socket --vga serial0 --agent 1
qm template 9000
```

## Koppelen

1. Ga naar **Beheer → Koppelingen** en kies **Koppeling toevoegen**, met als platform **Proxmox VE**.
2. Vul het adres in, bijvoorbeeld `https://pve.jouwbedrijf.nl:8006`, en het token in de vorm `gebruiker@realm!naam` met het geheim.
3. Vul de node, bridge en opslag in, en de IP-reeks met gateway en prefix. Stacklane deelt de adressen uit van laag naar hoog en geeft er nooit een twee keer uit.
4. Zet per regel een template en de naam die klanten zien, zoals `9000 Debian 13`.

Stacklane test de verbinding meteen. Het token wordt versleuteld opgeslagen met `SECRET_KEY`. **Certificaat controleren** staat standaard aan. Gebruikt Proxmox een zelfondertekend certificaat, geef de node dan een geldig certificaat, bijvoorbeeld via ACME onder *Node → Certificaten*. Zet je de controle uit, dan kan iedereen op het netwerk tussen Stacklane en Proxmox het token onderscheppen.

## Een product maken

Onder **Beheer → Producten** kies je als soort **VPS**, de koppeling, en het aantal processors, het geheugen en de schijf. Klanten zien alle actieve VPS-producten bij **VPS → Servers → Server bestellen**.

## Bestellen en leveren

1. De klant kiest een pakket, een naam, een besturingssysteem en eventueel SSH-sleutels.
2. Stacklane maakt een factuur. Na betaling kloont Stacklane de template, stelt cloud-init in met het IP-adres, een rootwachtwoord en de sleutels, vergroot de schijf, zet de firewall aan met een IP-filter op het eigen adres van de server en start de server. Zo kan een klant met root op zijn server niet het IP-adres van een andere klant op dezelfde bridge overnemen.
3. Een bestelling die na de vervaldatum nog 14 dagen open staat, vervalt vanzelf.

## Wat klanten kunnen

- de server starten, netjes afsluiten, hard uitzetten en herstarten
- het rootwachtwoord bekijken
- tot drie snapshots maken, terugzetten en verwijderen
- firewallregels toevoegen en verwijderen, per server of via een firewall voor meerdere servers

### Herbouwen

Onder **Herbouwen** op de serverpagina installeert een klant de server opnieuw met een van de besturingssystemen uit de koppeling, met eventueel nieuwe SSH-sleutels. Alles op de systeemschijf gaat verloren; de klant krijgt een nieuw rootwachtwoord. Volumes, IP-adressen, floating IPs, netwerken en firewallregels blijven staan. Tijdens het herbouwen kan de klant niets anders met de server doen.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | Stacklane kloont de template naar een tijdelijke VM, zet de schijf daarvan als `scsi0` in de server en verwijdert pas daarna de oude schijf. Mislukt het verplaatsen, dan komt de oude schijf terug. De tijdelijke VM wordt altijd opgeruimd. |
| OpenStack | De *rebuild*-actie van Nova (API 2.57 of nieuwer), met nieuwe cloud-init-gegevens. |
| VirtFusion | Herbouwen gaat in het panel van VirtFusion. |

### Grafieken en rescue

Onder **Grafieken** ziet de klant het processor-, geheugen-, netwerk- en schijfgebruik van het afgelopen uur, de afgelopen dag, week of maand. Stacklane haalt die cijfers bij Proxmox VE op (`rrddata`); OpenStack en VirtFusion geven ze niet door.

Onder **Rescue** start een klant zijn server in een tijdelijke herstelomgeving, met een eenmalig wachtwoord om met SSH in te loggen. De eigen schijf blijft gekoppeld, zodat hij een kapotte configuratie kan repareren of bestanden kan terughalen. Dit werkt op OpenStack (de *rescue*-actie van Nova); rescue en een gekoppelde ISO sluiten elkaar uit, omdat OpenStack beide met dezelfde rescue-modus doet. Op Proxmox VE en VirtFusion is er nog geen rescue-modus.

### Reverse DNS

Klanten zetten onder **Reverse DNS** op de serverpagina zelf een PTR-record voor het IP-adres van hun server en voor de floating IPs die eraan gekoppeld zijn. Daarvoor koppel je onder **Beheer → Koppelingen** een **PowerDNS**-server: het adres van de API (bijvoorbeeld `https://dns.jouwbedrijf.nl:8081`), de `api-key` uit `pdns.conf` en de server-ID (meestal `localhost`). Zet de API alleen open voor het panel en bij voorkeur achter https.

Stacklane maakt zelf geen zones aan. Het zoekt bij elk IP-adres de reverse-zone die PowerDNS al beheert (bijvoorbeeld `2.0.192.in-addr.arpa.`) en zet daarin alleen het PTR-record van dat ene adres. Voor IP-adressen zonder zo'n zone ziet de klant dat het via support gaat.

Een klant kan alleen een hostnaam instellen die met een A-record naar hetzelfde IP-adres wijst; zo kan niemand een PTR-record naar andermans domein laten wijzen. Wordt een server of floating IP opgeheven, dan haalt Stacklane het PTR-record weg, zodat de volgende klant met dat adres schoon begint.

### Firewalls

Onder **VPS → Firewalls** maakt een klant een firewall: een set regels met een naam, die hij aan een of meer servers koppelt. Past hij de regels aan, dan zet Stacklane ze meteen op alle gekoppelde servers. Met **Automatisch toewijzen** krijgt elke nieuwe server de firewall vanzelf zodra hij klaar is.

Stacklane zet de regels van een firewall bovenaan de firewall van de server en herkent ze aan een korte code in de notitie, zoals `[fw:1a2b3c4d]`. Eigen regels die de klant per server maakt blijven staan. Op de serverpagina staan de regels van een firewall met het label *Via een firewall*; die pas je aan op de firewallpagina.

| Platform | Wat werkt |
| --- | --- |
| Proxmox VE | Alle regels, ook blokkeren. Een poortlijst zoals 80,443 blijft één regel. |
| OpenStack | Alleen toestaan. Een poortlijst wordt per poort een aparte regel in de security group. |
| VirtFusion | Geen firewalls van Stacklane; de server staat niet in de lijst. |

Een klant heeft hoogstens 20 firewalls. Een regel heeft hoogstens 15 poorten of reeksen, een firewall samen hoogstens 100, en een server hoogstens 100 regels. Zo kan één klant het quotum voor security groups van een gedeeld OpenStack-project niet opmaken.

Lukt het bijwerken van een server niet, dan slaat Stacklane de firewall toch op en toont het welke server niet is bijgewerkt en waarom.

### Volumes

Een volume is extra schijfruimte voor een server. Maak onder **Beheer → Producten** een product van de soort **Volume** op een Proxmox- of OpenStack-koppeling, met een prijs per GB per maand. De klant bestelt onder **VPS → Volumes** een volume van 10 GB tot 1 TB voor een van zijn servers en betaalt de prijs per GB keer de grootte. Een klant heeft hoogstens tien volumes.

Na betaling maakt Stacklane het volume aan en koppelt het aan de server. De klant kan het loskoppelen en aan een andere server op dezelfde koppeling hangen.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | Een extra schijf (`scsi1` en verder) op de opslag uit de koppeling. Losgekoppeld blijft hij als `unused` bij de oude VM staan; koppelen aan een andere VM gaat met *move disk*. Wordt de VM opgeheven, dan verdwijnt het volume mee en zegt Stacklane het abonnement op. |
| OpenStack | Een volume in Cinder (blokopslag), gekoppeld via Nova. Wordt de server opgeheven, dan blijft het volume los bestaan. |
| VirtFusion | Geen volumes. |

Vergroten kan nog niet. Een volume opzeggen gaat via het abonnement.

### Floating IPs

Een floating IP is een vast IP-adres dat de klant van de ene server naar de andere verplaatst, bijvoorbeeld om zonder DNS-wijziging naar een nieuwe server over te stappen. Maak onder **Beheer → Producten** een product van de soort **Floating IP** op een Proxmox- of OpenStack-koppeling, met een prijs per IP per maand. De klant bestelt onder **VPS → Floating IPs** een of meer adressen; elk adres krijgt een eigen abonnement, zodat hij ze los kan opzeggen. Een klant heeft hoogstens tien floating IPs.

Na betaling reserveert Stacklane het adres. De klant koppelt het aan een server, verplaatst het naar een andere server op dezelfde koppeling, of koppelt het los. Een losgekoppeld adres blijft van de klant zolang het abonnement loopt. Bij opschorten haalt Stacklane het adres van de server en zet het terug zodra de dienst weer actief is.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | Het adres komt uit dezelfde IP-reeks als de servers; Stacklane geeft het nooit aan een nieuwe server. Koppelen zet het adres in het IP-filter (`ipfilter-net0`) van de VM, zodat alleen die VM het mag gebruiken. De klant stelt het adres zelf in op de server; het panel toont de commando's. |
| OpenStack | Een floating IP in Neutron op het **externe netwerk voor floating IPs** uit de koppeling, gekoppeld aan de poort van de server. Daarvoor moet het servernetwerk via een router aan dat externe netwerk hangen. De klant hoeft op de server niets in te stellen. Zonder extern netwerk in de koppeling verkoopt Stacklane op OpenStack geen floating IPs. |
| VirtFusion | Geen floating IPs. VirtFusion geeft een losgekoppeld adres meteen terug aan de pool, dus Stacklane kan het niet voor de klant vasthouden. |

Op Proxmox stelt de klant het adres zo in, met het eigen adres in plaats van het voorbeeld:

```sh
ip address add 192.0.2.15/32 dev eth0
arping -U -c 3 -I eth0 192.0.2.15
```

Het tweede commando laat de router weten dat het adres nu bij deze server zit; zonder dat kan het na een verplaatsing even duren voordat verkeer aankomt.

### Netwerken

Met een privénetwerk praten servers van één klant onderling, buiten het publieke netwerk om. De klant maakt onder **VPS → Netwerken** een netwerk met een naam en een privé IP-bereik (bijvoorbeeld `10.1.0.0/24`, met een netmasker van /16 tot /28) en koppelt daar servers op hetzelfde platform aan. Elke server krijgt een extra netwerkkaart met een adres uit dat bereik. Netwerken zijn gratis, maar alleen voor klanten met een actieve VPS op dat platform. Een klant heeft hoogstens tien netwerken, een server zit in hoogstens drie, en de bereiken van één klant mogen elkaar niet overlappen.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | Stel bij de koppeling een **bridge voor privénetwerken** in, bijvoorbeeld `vmbr1`, en een reeks VLAN's (standaard 2000 tot en met 2999). Elk netwerk krijgt een eigen VLAN uit die reeks; een server krijgt een extra kaart (`net1` en verder) op die bridge met die VLAN-tag. Stacklane kiest het adres en zet het in cloud-init, dus het staat er na de volgende herstart vanzelf op; de klant kan het ook meteen zelf instellen. Laat de bridge alle nodes verbinden als je er meer dan één hebt. Zonder bridge kunnen klanten op deze koppeling geen netwerken maken. |
| OpenStack | Een eigen netwerk en subnet in Neutron, zonder gateway en zonder security groups, zodat de servers elkaar vrij bereiken. Koppelen voegt via Nova een netwerkkaart toe; het adres komt via DHCP. |
| VirtFusion | Geen netwerken. |

Een netwerk verwijderen kan pas als er geen servers meer aan hangen. Wordt een server opgeheven, dan verdwijnt hij vanzelf uit zijn netwerken.

### ISO's

Een klant kan onder **VPS → ISO's** twee eigen ISO's importeren via een publieke https-link, met BIOS of UEFI. Stacklane haalt de ISO op de achtergrond binnen. Daarna kiest de klant op de serverpagina onder **ISO** welke ISO hij koppelt; de server herstart en start vanaf de ISO. Na het loskoppelen start hij weer van de eigen schijf.

Stacklane weigert links naar privé-adressen, localhost, link-local adressen en IPv6-adressen die naar zo'n adres vertalen, zodat niemand via een ISO-import je interne netwerk kan aftasten. De download zelf doet de hypervisor, en die volgt een doorverwijzing of een DNS-naam die later naar een ander adres wijst. Laat de hypervisor daarom niet bij je beheernetwerk of andere interne diensten komen; een uitgaande firewall op de node is genoeg. Een download die vastloopt, stopt Stacklane na een uur en ruimt het halve bestand op. ISO's die dertig dagen niet gekoppeld zijn, ruimt Stacklane zelf op.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | De ISO komt op de **opslag voor ISO's** uit de koppeling (standaard `local`, met het inhoudstype ISO-image). Het token heeft daarvoor `Datastore.AllocateTemplate` op die opslag nodig en `Sys.AccessNetwork` op de node (op oudere versies `Sys.Modify` op `/`). Koppelen zet de ISO als cd-rom vooraan in de opstartvolgorde. De firmware van de server volgt de ISO en blijft zo na het loskoppelen, zodat een vanaf de ISO geïnstalleerd systeem opstart. |
| OpenStack | Een privé-image in Glance, geïmporteerd met *web-download*. Koppelen start de server in rescue-modus met de ISO als cd-rom (Nova API 2.87 of nieuwer); loskoppelen haalt hem uit rescue. |
| VirtFusion | Geen ISO's. |

### Gebruikers

Een klant kan onder **VPS → Gebruikers** anderen toegang geven tot zijn VPS-omgeving, bijvoorbeeld een collega of een beheerder. Die persoon heeft een eigen Stacklane-account met een bevestigd e-mailadres nodig, en de klant zelf ook; de klant vult het e-mailadres in en kiest de rechten. Er zijn 49 rechten in negen groepen, van *Servers → Start* tot *Firewalls → Regels verwijderen*.

De gebruiker krijgt een uitnodiging en vindt die onder **Gedeelde toegang**. Pas als hij op **Accepteren** klikt, krijgt hij toegang en ziet de eigenaar zijn naam; tot dan staat hij als *Uitgenodigd* in de lijst. Met **Openen** werkt hij in de omgeving van de eigenaar; het menu toont dan alleen wat hij mag. Stacklane controleert elk recht op de server, niet alleen in het scherm.

- Bestellen, betalen en het rootwachtwoord blijven bij de eigenaar.
- Iemand met het recht *Gebruikers beheren* kan anderen alleen rechten geven die hij zelf heeft.
- Een klant geeft hoogstens 25 mensen toegang. Intrekken werkt meteen, en de gebruiker kan zelf zijn toegang opgeven.
- Via de API werkt het ook: stuur de header `X-Vps-Owner` met het ID van de eigenaar mee bij aanroepen onder `/api/vps` en `/api/ssh-keys`.

## OpenStack

Bij OpenStack regelt de cloud zelf de IP-adressen, de schijf en de firewall. Stacklane praat met Keystone, Nova, Glance en Neutron.

Je hebt nodig:

- een **application credential** van het project waarin de servers komen: `openstack application credential create stacklane`
- de ID van het netwerk waaraan de servers komen: `openstack network list`
- voor floating IPs: de ID van het externe netwerk waar ze vandaan komen: `openstack network list --external`
- de ID's van de images die klanten mogen kiezen: `openstack image list`
- een of meer flavors

Koppel onder **Beheer → Koppelingen** met als platform **OpenStack**. Vul het adres van Keystone in, bijvoorbeeld `https://keystone.jouwbedrijf.nl:5000/v3`, de credential, eventueel de regio, het netwerk, eventueel het externe netwerk voor floating IPs, en per regel een image-ID met de naam die klanten zien. Bij een VPS-product op deze koppeling kies je daarna een flavor; Stacklane haalt de lijst live op.

Verschillen met Proxmox:

- elke server krijgt een eigen security group, die eerst al het inkomende verkeer toestaat
- in de firewall kunnen klanten alleen regels maken die verkeer toestaan, met één poort of reeks per regel
- een snapshot is een image in Glance; terugzetten herbouwt de server vanaf dat image
- het panel toont geen live processor- en geheugengebruik
- bij opheffen ruimt Stacklane de server, de security group en de snapshots op

## VirtFusion

VirtFusion heeft een eigen panel voor klanten. Stacklane regelt de bestelling, de facturen en het aanmaken, en stuurt klanten voor de rest door naar VirtFusion.

Je hebt nodig:

- een API-token onder **Configuration → API**, met rechten op gebruikers, servers, pakketten en SSH-sleutels
- het nummer van de hypervisorgroep waarop nieuwe servers komen
- de nummers van de templates die klanten mogen kiezen
- een of meer pakketten

Koppel onder **Beheer → Koppelingen** met als platform **VirtFusion**. Vul het adres van je VirtFusion-panel in, het token, de groep en per regel een template-nummer met een naam, zoals `8 Debian 12`. Bij een VPS-product kies je daarna een pakket van VirtFusion.

Na betaling maakt Stacklane de klant aan in VirtFusion (als die nog niet bestaat), zet zijn SSH-sleutels erbij, maakt de server en installeert het besturingssysteem. In Stacklane kan de klant de server aan- en uitzetten en herstarten. Met **Open in VirtFusion** logt hij in één klik in op het panel van VirtFusion; daar vindt hij het rootwachtwoord, de console en back-ups. Voor die inloglink is een commerciële licentie van VirtFusion nodig.

Opschorten en heractiveren gebeurt in VirtFusion zelf. Opheffen verwijdert de server meteen.

## Lokaal testen

In de repository staat `scripts/proxmox/dev.sh`. Dat zet een Proxmox VE in een VM op je eigen machine, met 2 GB geheugen. Zet hem uit met `scripts/proxmox/dev.sh down` als je klaar bent.
