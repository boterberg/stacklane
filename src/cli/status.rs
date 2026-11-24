use std::process::Command;

use super::{banner, done, fail, readiness, warn};

fn systemctl(args: &[&str]) -> String {
    Command::new("systemctl")
        .args(args)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default()
}

pub async fn run() -> anyhow::Result<()> {
    banner("Status");

    match systemctl(&["is-active", "stacklane"]).as_str() {
        "active" => done("Service draait"),
        state => fail(&format!(
            "Service: {}",
            if state.is_empty() { "onbekend" } else { state }
        )),
    }

    match readiness() {
        Some(version) => done(&format!("Database bereikbaar, versie {version}")),
        None => fail("Geen antwoord op /readyz"),
    }

    if super::apt_source_configured() {
        done("Pakketbron stacklane.io/apt ingesteld");
    } else {
        warn("Geen Stacklane-pakketbron, updates komen niet binnen");
    }

    if std::env::var("DEMO_MODE").is_ok_and(|value| value == "true") {
        let next = systemctl(&[
            "show",
            "stacklane-demo-reset.timer",
            "--property=NextElapseUSecRealtime",
            "--value",
        ]);
        done(&format!(
            "Demomodus aan, volgende reset: {}",
            if next.is_empty() { "geen timer" } else { &next }
        ));
    }

    if let Ok(snippet) = std::fs::read_to_string("/etc/caddy/stacklane.caddy")
        && let Some(domain) = snippet.split_whitespace().next()
    {
        done(&format!("Webadres https://{domain}"));
    }
    println!();
    Ok(())
}
