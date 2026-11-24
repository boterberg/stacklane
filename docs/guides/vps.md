# VPS met Proxmox VE

Stacklane maakt virtuele servers aan op je eigen Proxmox VE-cluster. Klanten bestellen een server in het panel, betalen de factuur, en binnen een paar minuten draait hij. Daarna zetten ze hem zelf aan en uit, maken ze snapshots en stellen ze de firewall in.

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
- firewallregels toevoegen en verwijderen

## Lokaal testen

In de repository staat `scripts/proxmox/dev.sh`. Dat zet een Proxmox VE in een VM op je eigen machine, met 2 GB geheugen. Zet hem uit met `scripts/proxmox/dev.sh down` als je klaar bent.
