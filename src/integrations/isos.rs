use std::net::IpAddr;

use anyhow::{Context, bail};

use super::Driver;
use super::openstack::OpenStack;
use super::proxmox::Proxmox;

pub enum IsoStore {
    Proxmox(Proxmox),
    OpenStack(OpenStack),
}

fn vmid(value: &str) -> anyhow::Result<u32> {
    value.parse().context("ongeldig VM-nummer")
}

impl IsoStore {
    pub fn from_driver(driver: Driver) -> Option<Self> {
        match driver {
            Driver::Proxmox(client) => Some(Self::Proxmox(client)),
            Driver::OpenStack(client) => Some(Self::OpenStack(client)),
            _ => None,
        }
    }

    pub async fn import(
        &self,
        file: &str,
        name: &str,
        url: &str,
        uefi: bool,
    ) -> anyhow::Result<String> {
        match self {
            Self::Proxmox(client) => {
                client.import_iso(file, url).await?;
                Ok(file.to_owned())
            }
            Self::OpenStack(client) => client.import_iso(name, url, uefi).await,
        }
    }

    pub async fn delete(&self, external_id: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.delete_iso(external_id).await,
            Self::OpenStack(client) => client.delete_iso(external_id).await,
        }
    }

    pub async fn mount(&self, server: &str, external_id: &str, uefi: bool) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.mount_iso(vmid(server)?, external_id, uefi).await,
            Self::OpenStack(client) => client.mount_iso(server, external_id).await,
        }
    }

    pub async fn unmount(&self, server: &str) -> anyhow::Result<()> {
        match self {
            Self::Proxmox(client) => client.unmount_iso(vmid(server)?).await,
            Self::OpenStack(client) => client.unmount_iso(server).await,
        }
    }
}

fn embedded_v4(ip: std::net::Ipv6Addr) -> Option<std::net::Ipv4Addr> {
    let segments = ip.segments();
    let [.., a, b, c, d] = ip.octets();
    let last = std::net::Ipv4Addr::new(a, b, c, d);
    if let Some(mapped) = ip.to_ipv4_mapped() {
        Some(mapped)
    } else if segments[..6] == [0; 6]
        || segments[..6] == [0x64, 0xff9b, 0, 0, 0, 0]
        || segments[..3] == [0x64, 0xff9b, 1]
    {
        Some(last)
    } else if segments[0] == 0x2002 {
        let [high, low] = [segments[1], segments[2]];
        Some(std::net::Ipv4Addr::from(
            (u32::from(high) << 16) | u32::from(low),
        ))
    } else {
        None
    }
}

fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || ip.is_documentation()
                || a == 0
                || (a == 100 && (64..128).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && (18..20).contains(&b))
                || a >= 224)
        }
        IpAddr::V6(ip) => {
            let [first, second, ..] = ip.segments();
            if let Some(v4) = embedded_v4(ip) {
                return public(IpAddr::V4(v4));
            }
            !(ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                || (first & 0xffc0) == 0xfec0
                || (first == 0x2001 && second == 0)
                || (first == 0x2001 && second == 0x0db8)
                || (first == 0x0100 && second == 0))
        }
    }
}

pub async fn check_url(raw: &str) -> anyhow::Result<String> {
    let url = reqwest::Url::parse(raw.trim()).context("dit is geen geldige URL")?;
    if url.scheme() != "https" {
        bail!("gebruik een https-URL");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("een URL met inloggegevens kan niet");
    }
    let host = url.host_str().context("de URL heeft geen host")?.to_owned();
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses: Vec<IpAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .context("de host van de URL bestaat niet")?
        .map(|address| address.ip())
        .collect();
    if addresses.is_empty() || !addresses.iter().copied().all(public) {
        bail!("de URL moet naar een publiek adres op internet wijzen");
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::public;

    #[test]
    fn only_public_addresses_pass() {
        for ip in [
            "10.0.0.1",
            "192.168.1.1",
            "127.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "::ffff:10.0.0.1",
            "64:ff9b::a9fe:a9fe",
            "64:ff9b:1::a00:1",
            "2002:a9fe:a9fe::1",
            "::a9fe:a9fe",
            "2001:0:4136:e378:8000:63bf:3fff:fdd2",
            "fec0::1",
            "192.0.0.1",
            "2001:db8::1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "1.1.1.1",
            "93.190.187.230",
            "2a10:92c0::1",
            "64:ff9b::101:101",
            "2002:101:101::1",
        ] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }
}
