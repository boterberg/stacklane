#!/bin/sh
set -eu

BASE=${STACKLANE_PVE_DIR:-$HOME/.local/share/stacklane-pve}
IMAGE_URL=https://cloud.debian.org/images/cloud/trixie/latest/debian-13-generic-amd64.qcow2
API_PORT=18006
SSH_PORT=2223
MEMORY=${STACKLANE_PVE_MEMORY:-2048}

log() {
  printf '  %s\n' "$*"
}

guest() {
  ssh -q -i "$BASE/id_ed25519" -p "$SSH_PORT" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=5 root@127.0.0.1 "$@"
}

running() {
  [ -s "$BASE/qemu.pid" ] && kill -0 "$(cat "$BASE/qemu.pid")" 2>/dev/null
}

prepare() {
  mkdir -p "$BASE"
  [ -s "$BASE/id_ed25519" ] || ssh-keygen -q -t ed25519 -N '' -C stacklane-pve -f "$BASE/id_ed25519"
  if [ ! -s "$BASE/debian-13.qcow2" ]; then
    log "Debian 13 cloud-image downloaden"
    curl -fsSL -o "$BASE/debian-13.qcow2.part" "$IMAGE_URL"
    mv "$BASE/debian-13.qcow2.part" "$BASE/debian-13.qcow2"
  fi
  if [ ! -s "$BASE/disk.qcow2" ]; then
    qemu-img create -q -f qcow2 -F qcow2 -b "$BASE/debian-13.qcow2" "$BASE/disk.qcow2" 40G
  fi
  if [ ! -s "$BASE/seed.iso" ]; then
    mkdir -p "$BASE/seed"
    printf 'instance-id: stacklane-pve\nlocal-hostname: pve\n' > "$BASE/seed/meta-data"
    cat > "$BASE/seed/user-data" <<USERDATA
#cloud-config
disable_root: false
ssh_pwauth: false
users:
  - name: root
    ssh_authorized_keys:
      - $(cat "$BASE/id_ed25519.pub")
USERDATA
    genisoimage -quiet -output "$BASE/seed.iso" -volid cidata -joliet -rock "$BASE/seed/user-data" "$BASE/seed/meta-data"
  fi
}

boot() {
  running && return
  qemu-system-x86_64 -enable-kvm -cpu host -smp 4 -m "$MEMORY" -name stacklane-pve \
    -drive file="$BASE/disk.qcow2",if=virtio \
    -drive file="$BASE/seed.iso",if=virtio,media=cdrom \
    -nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:$API_PORT-:8006,hostfwd=tcp:127.0.0.1:$SSH_PORT-:22 \
    -display none -daemonize -pidfile "$BASE/qemu.pid" -serial file:"$BASE/console.log"
  log "wachten tot de VM via SSH bereikbaar is"
  for _ in $(seq 1 90); do
    guest true 2>/dev/null && return
    sleep 2
  done
  echo "de VM start niet, kijk in $BASE/console.log" >&2
  exit 1
}

install_pve() {
  if guest test -f /etc/pve/.version 2>/dev/null; then
    return
  fi
  log "Proxmox VE installeren, dit duurt een paar minuten"
  guest sh -s <<'INSTALL'
set -eu
export DEBIAN_FRONTEND=noninteractive
sed -i '/ pve$/d' /etc/hosts
echo "10.0.2.15 pve.stacklane.test pve" >> /etc/hosts
hostnamectl set-hostname pve
apt-get update -qq
apt-get install -y -qq curl ca-certificates >/dev/null
curl -fsSL https://enterprise.proxmox.com/debian/proxmox-archive-keyring-trixie.gpg -o /usr/share/keyrings/proxmox-archive-keyring.gpg
cat > /etc/apt/sources.list.d/pve-install-repo.sources <<SOURCES
Types: deb
URIs: http://download.proxmox.com/debian/pve
Suites: trixie
Components: pve-no-subscription
Signed-By: /usr/share/keyrings/proxmox-archive-keyring.gpg
SOURCES
echo "postfix postfix/main_mailer_type select No configuration" | debconf-set-selections
apt-get update -qq
apt-get install -y -qq proxmox-ve postfix open-iscsi chrony >/dev/null
rm -f /etc/apt/sources.list.d/pve-enterprise.sources /etc/apt/sources.list.d/ceph.sources
INSTALL
  guest systemctl reboot || true
  sleep 10
  for _ in $(seq 1 90); do
    guest test -f /etc/pve/.version 2>/dev/null && break
    sleep 2
  done
}

network() {
  guest sh -s <<'NETWORK'
set -eu
if ! grep -q vmbr0 /etc/network/interfaces; then
  cat >> /etc/network/interfaces <<IFACES

auto vmbr0
iface vmbr0 inet static
    address 10.10.10.1/24
    bridge-ports none
    bridge-stp off
    bridge-fd 0
    post-up echo 1 > /proc/sys/net/ipv4/ip_forward
    post-up iptables -t nat -A POSTROUTING -s 10.10.10.0/24 -o eth0 -j MASQUERADE
    post-down iptables -t nat -D POSTROUTING -s 10.10.10.0/24 -o eth0 -j MASQUERADE
IFACES
  ifreload -a
fi
pvesm set local --content images,iso,vztmpl,backup,rootdir,snippets
NETWORK
}

template() {
  if guest qm status 9000 >/dev/null 2>&1; then
    return
  fi
  log "cloud-init-template 9000 maken"
  guest mkdir -p /var/lib/vz/template/qcow
  scp -q -i "$BASE/id_ed25519" -P "$SSH_PORT" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "$BASE/debian-13.qcow2" root@127.0.0.1:/var/lib/vz/template/qcow/debian-13.qcow2
  guest sh -s <<'TEMPLATE'
set -eu
qm create 9000 --name debian-13 --memory 512 --cores 1 --net0 virtio,bridge=vmbr0 --scsihw virtio-scsi-pci --ostype l26 --agent 1 --serial0 socket --vga serial0
qm set 9000 --scsi0 local:0,import-from=/var/lib/vz/template/qcow/debian-13.qcow2,format=qcow2
qm set 9000 --ide2 local:cloudinit --boot order=scsi0
qm template 9000
TEMPLATE
}

token() {
  if [ -s "$BASE/token.env" ]; then
    return
  fi
  guest sh -s > "$BASE/token.env" <<'TOKEN'
set -eu
pveum user add stacklane@pve --comment "Stacklane test" >/dev/null 2>&1 || true
pveum acl modify / --users stacklane@pve --roles Administrator
secret=$(pveum user token add stacklane@pve panel --privsep 0 --output-format json | sed -n 's/.*"value":"\([^"]*\)".*/\1/p')
echo "PVE_URL=https://127.0.0.1:18006"
echo "PVE_TOKEN_ID=stacklane@pve!panel"
echo "PVE_TOKEN_SECRET=$secret"
echo "PVE_NODE=pve"
TOKEN
  chmod 600 "$BASE/token.env"
}

case "${1:-}" in
  up)
    prepare
    boot
    install_pve
    network
    template
    token
    log "Proxmox draait op https://127.0.0.1:$API_PORT, API-token staat in $BASE/token.env"
    ;;
  down)
    if running; then
      guest poweroff 2>/dev/null || kill "$(cat "$BASE/qemu.pid")"
      log "VM gestopt"
    fi
    if [ "${2:-}" = "--purge" ]; then
      rm -rf "$BASE"
      log "alles weggegooid"
    fi
    ;;
  ssh)
    shift
    guest "$@"
    ;;
  *)
    echo "gebruik: $0 up | down [--purge] | ssh [commando]" >&2
    exit 1
    ;;
esac
