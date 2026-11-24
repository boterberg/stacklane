# VPS met Proxmox VE, OpenStack of VirtFusion

Stacklane maakt virtuele servers aan op je eigen Proxmox VE-cluster, in een OpenStack-cloud of via VirtFusion. Klanten bestellen een server in het panel, betalen de factuur, en binnen een paar minuten draait hij. Daarna zetten ze hem zelf aan en uit, maken ze snapshots en stellen ze de firewall in.

## Wat je nodig hebt

- Proxmox VE 8 of 9
- Een **API-token** met rechten op de VM's en de opslag, bijvoorbeeld de rol `PVEVMAdmin` plus `Datastore.AllocateSpace` op je opslag
- Eén of meer **cloud-init-templates**, bijvoorbeeld van de officiële cloud-images van Debian of Ubuntu
- Een reeks vrije IPv4-adressen op de bridge waar de servers aan komen

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

Stacklane test de verbinding meteen. Het token wordt versleuteld opgeslagen met `SECRET_KEY`. Gebruikt Proxmox een eigen certificaat, laat **Certificaat controleren** dan uit.

## Een product maken

Onder **Beheer → Producten** kies je als soort **VPS**, de koppeling, en het aantal processors, het geheugen en de schijf. Klanten zien alle actieve VPS-producten bij **VPS → Servers → Server bestellen**.

## Bestellen en leveren

1. De klant kiest een pakket, een naam, een besturingssysteem en eventueel SSH-sleutels.
2. Stacklane maakt een factuur. Na betaling kloont Stacklane de template, stelt cloud-init in met het IP-adres, een rootwachtwoord en de sleutels, vergroot de schijf, zet de firewall aan en start de server.
3. Een bestelling die na de vervaldatum nog 14 dagen open staat, vervalt vanzelf.

## Wat klanten kunnen

- de server starten, netjes afsluiten, hard uitzetten en herstarten
- het rootwachtwoord bekijken
- tot drie snapshots maken, terugzetten en verwijderen
- firewallregels toevoegen en verwijderen, per server of via een firewall voor meerdere servers

### Firewalls

Onder **VPS → Firewalls** maakt een klant een firewall: een set regels met een naam, die hij aan een of meer servers koppelt. Past hij de regels aan, dan zet Stacklane ze meteen op alle gekoppelde servers. Met **Automatisch toewijzen** krijgt elke nieuwe server de firewall vanzelf zodra hij klaar is.

Stacklane zet de regels van een firewall bovenaan de firewall van de server en herkent ze aan een korte code in de notitie, zoals `[fw:1a2b3c4d]`. Eigen regels die de klant per server maakt blijven staan. Op de serverpagina staan de regels van een firewall met het label *Via een firewall*; die pas je aan op de firewallpagina.

| Platform | Wat werkt |
| --- | --- |
| Proxmox VE | Alle regels, ook blokkeren. Een poortlijst zoals 80,443 blijft één regel. |
| OpenStack | Alleen toestaan. Een poortlijst wordt per poort een aparte regel in de security group. |
| VirtFusion | Geen firewalls van Stacklane; de server staat niet in de lijst. |

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

### ISO's

Een klant kan onder **VPS → ISO's** twee eigen ISO's importeren via een publieke https-link, met BIOS of UEFI. Stacklane haalt de ISO op de achtergrond binnen. Daarna kiest de klant op de serverpagina onder **ISO** welke ISO hij koppelt; de server herstart en start vanaf de ISO. Na het loskoppelen start hij weer van de eigen schijf.

Stacklane weigert links naar privé-adressen, localhost en link-local adressen, zodat niemand via een ISO-import je interne netwerk kan aftasten. ISO's die dertig dagen niet gekoppeld zijn, ruimt Stacklane zelf op.

| Platform | Hoe het werkt |
| --- | --- |
| Proxmox VE | De ISO komt op de **opslag voor ISO's** uit de koppeling (standaard `local`, met het inhoudstype ISO-image). Het token heeft daarvoor `Datastore.AllocateTemplate` op die opslag nodig en `Sys.AccessNetwork` op de node (op oudere versies `Sys.Modify` op `/`). Koppelen zet de ISO als cd-rom vooraan in de opstartvolgorde. De firmware van de server volgt de ISO en blijft zo na het loskoppelen, zodat een vanaf de ISO geïnstalleerd systeem opstart. |
| OpenStack | Een privé-image in Glance, geïmporteerd met *web-download*. Koppelen start de server in rescue-modus met de ISO als cd-rom (Nova API 2.87 of nieuwer); loskoppelen haalt hem uit rescue. |
| VirtFusion | Geen ISO's. |

### Gebruikers

Een klant kan onder **VPS → Gebruikers** anderen toegang geven tot zijn VPS-omgeving, bijvoorbeeld een collega of een beheerder. Die persoon heeft een eigen Stacklane-account nodig; de klant vult het e-mailadres in en kiest de rechten. Er zijn 49 rechten in negen groepen, van *Servers → Start* tot *Firewalls → Regels verwijderen*.

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
- de ID's van de images die klanten mogen kiezen: `openstack image list`
- een of meer flavors

Koppel onder **Beheer → Koppelingen** met als platform **OpenStack**. Vul het adres van Keystone in, bijvoorbeeld `https://keystone.jouwbedrijf.nl:5000/v3`, de credential, eventueel de regio, het netwerk, en per regel een image-ID met de naam die klanten zien. Bij een VPS-product op deze koppeling kies je daarna een flavor; Stacklane haalt de lijst live op.

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
