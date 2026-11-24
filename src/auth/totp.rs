use hmac::{Hmac, KeyInit, Mac};
use qrcode::QrCode;
use qrcode::render::svg;
use sha1::Sha1;

const STEP: i64 = 30;
const DIGITS: u32 = 6;
const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn new_secret() -> Vec<u8> {
    rand::random::<[u8; 20]>().to_vec()
}

pub fn base32(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(char::from(ALPHABET[((buffer >> bits) & 31) as usize]));
        }
    }
    if bits > 0 {
        out.push(char::from(ALPHABET[((buffer << (5 - bits)) & 31) as usize]));
    }
    out
}

fn code_at(secret: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("hmac accepts any key length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[digest.len() - 1] & 0x0f);
    let value = u32::from_be_bytes([
        digest[offset] & 0x7f,
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    value % 10u32.pow(DIGITS)
}

pub fn step(unix_time: i64) -> i64 {
    unix_time.div_euclid(STEP)
}

pub fn verify(secret: &[u8], code: &str, unix_time: i64, last_step: Option<i64>) -> Option<i64> {
    let code = code.trim().replace(' ', "");
    if code.len() != DIGITS as usize {
        return None;
    }
    let wanted: u32 = code.parse().ok()?;
    let now = step(unix_time);
    (now - 1..=now + 1)
        .filter(|candidate| last_step.is_none_or(|last| *candidate > last))
        .find(|candidate| {
            u64::try_from(*candidate).is_ok_and(|counter| code_at(secret, counter) == wanted)
        })
}

pub fn provisioning_uri(secret: &[u8], issuer: &str, account: &str) -> String {
    let encode = |value: &str| {
        value
            .bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    char::from(byte).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect::<String>()
    };
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&digits={DIGITS}&period={STEP}",
        encode(issuer),
        encode(account),
        base32(secret),
        encode(issuer)
    )
}

pub fn qr_svg(uri: &str) -> String {
    QrCode::new(uri.as_bytes())
        .map(|code| {
            code.render::<svg::Color>()
                .min_dimensions(180, 180)
                .quiet_zone(false)
                .build()
        })
        .unwrap_or_default()
}

pub fn recovery_code() -> String {
    let raw = base32(&rand::random::<[u8; 10]>()).to_lowercase();
    format!("{}-{}", &raw[..5], &raw[5..10])
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn matches_rfc_6238_vectors() {
        assert_eq!(code_at(RFC_SECRET, step(59) as u64), 287_082);
        assert_eq!(code_at(RFC_SECRET, step(1_111_111_109) as u64), 81_804);
        assert_eq!(code_at(RFC_SECRET, step(1_234_567_890) as u64), 5_924);
        assert_eq!(code_at(RFC_SECRET, step(2_000_000_000) as u64), 279_037);
    }

    #[test]
    fn accepts_neighbouring_steps_once() {
        let accepted = verify(RFC_SECRET, "287082", 59 + 30, None);
        assert_eq!(accepted, Some(1));
        assert_eq!(verify(RFC_SECRET, "287 082", 59, Some(1)), None);
        assert_eq!(verify(RFC_SECRET, "287082", 59 + 120, None), None);
        assert_eq!(verify(RFC_SECRET, "28708", 59, None), None);
    }

    #[test]
    fn encodes_base32_like_rfc_4648() {
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
        assert_eq!(base32(b"f"), "MY");
    }

    #[test]
    fn builds_an_otpauth_uri() {
        let uri = provisioning_uri(b"12345", "Stacklane Demo", "jan@example.nl");
        assert_eq!(
            uri,
            "otpauth://totp/Stacklane%20Demo:jan%40example.nl?secret=GEZDGNBV&issuer=Stacklane%20Demo&digits=6&period=30"
        );
        assert!(qr_svg(&uri).starts_with("<?xml"));
    }
}
