pub mod admin;
pub mod setup;
pub mod status;
pub mod update;

use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::Context;

use crate::config::VERSION;

const SYMBOL: [&str; 8] = [
    "▄███████████▄",
    "███▀▀▀▀▀▀▀▀▀▀",
    "███▄▄▄▄▄▄▄▄▄▄",
    "▀▀▀▀▀▀▀▀▀▀███",
    "▄▄▄▄▄▄▄▄▄▄███",
    "███▀▀▀▀▀▀▀▀▀▀",
    "███▄▄▄▄▄▄▄▄▄▄",
    "▀███████████▀",
];

fn colors_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn paint(code: &str, text: &str) -> String {
    if colors_enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

pub fn bold(text: &str) -> String {
    paint("1", text)
}

pub fn dim(text: &str) -> String {
    paint("2", text)
}

pub fn banner(subtitle: &str) {
    let side = [
        String::new(),
        bold(&format!("Stacklane {VERSION}")),
        dim("Klantpanel voor hostingbedrijven"),
        String::new(),
        subtitle.to_owned(),
    ];
    println!();
    for (index, row) in SYMBOL.iter().enumerate() {
        let text = side.get(index).cloned().unwrap_or_default();
        println!("  {}   {}", paint("38;2;99;91;255", row), text);
    }
    println!();
}

pub fn step(text: &str) {
    println!("  {} {text}", paint("38;2;99;91;255", "›"));
}

pub fn done(text: &str) {
    println!("  {} {text}", paint("32", "✓"));
}

pub fn warn(text: &str) {
    println!("  {} {text}", paint("33", "!"));
}

pub fn fail(text: &str) {
    println!("  {} {text}", paint("31", "✗"));
}

pub fn ask(question: &str, default: Option<&str>) -> anyhow::Result<String> {
    let hint = default
        .map(|value| format!(" {}", dim(&format!("[{value}]"))))
        .unwrap_or_default();
    print!("  {question}{hint}: ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    Ok(if answer.is_empty() {
        default.unwrap_or_default().to_owned()
    } else {
        answer.to_owned()
    })
}

pub fn confirm(question: &str) -> anyhow::Result<bool> {
    let answer = ask(&format!("{question} (j/n)"), Some("j"))?;
    Ok(matches!(
        answer.to_lowercase().as_str(),
        "j" | "ja" | "y" | "yes"
    ))
}

pub fn require_root() -> anyhow::Result<()> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .context("running id")?;
    anyhow::ensure!(
        String::from_utf8_lossy(&output.stdout).trim() == "0",
        "dit commando heeft root nodig, gebruik sudo"
    );
    Ok(())
}

pub fn exec(program: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("{program} starten"))?;
    anyhow::ensure!(
        output.status.success(),
        "{program} {} mislukte: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn installed(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn apt_source_configured() -> bool {
    [
        "/etc/apt/sources.list.d/stacklane.list",
        "/etc/apt/sources.list.d/stacklane.sources",
    ]
    .iter()
    .any(|path| std::path::Path::new(path).exists())
}

fn bind_addr() -> String {
    std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into())
}

pub fn readiness() -> Option<String> {
    use std::io::Read;
    let addr = bind_addr();
    let mut stream = std::net::TcpStream::connect(&addr).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    write!(
        stream,
        "GET /readyz HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    if !response.starts_with("HTTP/1.1 200") {
        return None;
    }
    let body = response.split("\r\n\r\n").nth(1)?;
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value["version"].as_str().map(str::to_owned)
}

pub fn wait_until_ready(seconds: u64) -> Option<String> {
    for _ in 0..seconds {
        if let Some(version) = readiness() {
            return Some(version);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    None
}

pub fn print_help() {
    banner("");
    println!("  {}", bold("Gebruik: stacklane <commando>"));
    println!();
    let commands = [
        (
            "setup",
            "Eerste installatie: database, domein, webserver en beheerder",
        ),
        (
            "update",
            "Back-up maken en bijwerken naar de nieuwste versie",
        ),
        ("status", "Versie, service, database en webadres bekijken"),
        ("admin create", "Een beheerdersaccount aanmaken"),
        ("serve", "De server starten (dit doet systemd)"),
        (
            "demo-reset",
            "Demogegevens terugzetten, alleen met DEMO_MODE=true",
        ),
        ("version", "Versie tonen"),
    ];
    for (name, description) in commands {
        println!("    {:<14}{}", bold(name), dim(description));
    }
    println!();
}
