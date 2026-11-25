use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::settings;
use crate::state::AppState;

const KINDS: [&str; 3] = ["logo", "logo_dark", "favicon"];
const MAX_BYTES: usize = 512 * 1024;
const MAX_NAME: usize = 60;
const MAX_CSS: usize = 64 * 1024;
const NEUTRALS: [&str; 5] = ["slate", "gray", "neutral", "stone", "tinted"];
const RADII: [&str; 2] = ["sharp", "round"];

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Branding {
    pub name: Option<String>,
    pub accent: Option<String>,
    pub accent_dark: Option<String>,
    pub neutral: Option<String>,
    pub radius: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeCss {
    pub css: String,
}

#[derive(Serialize)]
pub struct BrandingInfo {
    name: Option<String>,
    accent: Option<String>,
    accent_dark: Option<String>,
    neutral: Option<String>,
    radius: Option<String>,
    css: Option<String>,
    logo: Option<i64>,
    logo_dark: Option<i64>,
    favicon: Option<i64>,
}

#[derive(Deserialize)]
pub struct Upload {
    data: String,
}

fn sniff(bytes: &[u8], kind: &str) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if kind == "favicon" && bytes.starts_with(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else {
        None
    }
}

fn accent(value: &str) -> Option<String> {
    let hex = value.trim().trim_start_matches('#');
    (hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| format!("#{}", hex.to_lowercase()))
}

fn choice(value: Option<&str>, allowed: &[&str], field: &'static str) -> AppResult<Option<String>> {
    match value.map(str::trim) {
        None | Some("") | Some("zinc") | Some("default") => Ok(None),
        Some(value) if allowed.contains(&value) => Ok(Some(value.to_owned())),
        Some(_) => Err(AppError::field(field, "Kies een van de opties.")),
    }
}

fn color(value: Option<&str>, field: &'static str) -> AppResult<Option<String>> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) => accent(value)
            .map(Some)
            .ok_or_else(|| AppError::field(field, "Gebruik een kleur als #4f46e5.")),
    }
}

fn css_version(css: &str) -> Option<String> {
    use sha2::Digest;
    (!css.trim().is_empty()).then(|| hex::encode(&sha2::Sha256::digest(css.as_bytes())[..6]))
}

fn known(kind: &str) -> AppResult<()> {
    if KINDS.contains(&kind) {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

pub async fn info(state: &AppState) -> AppResult<BrandingInfo> {
    let branding: Branding = settings::get(&state.db, "branding").await?;
    let versions: Vec<(String, i64)> =
        sqlx::query_as("SELECT kind, extract(epoch FROM updated_at)::bigint FROM brand_assets")
            .fetch_all(&state.db)
            .await?;
    let version = |kind: &str| {
        versions
            .iter()
            .find(|(name, _)| name == kind)
            .map(|(_, version)| *version)
    };
    let theme: ThemeCss = settings::get(&state.db, "theme_css").await?;
    Ok(BrandingInfo {
        name: branding.name,
        accent: branding.accent,
        accent_dark: branding.accent_dark,
        neutral: branding.neutral,
        radius: branding.radius,
        css: css_version(&theme.css),
        logo: version("logo"),
        logo_dark: version("logo_dark"),
        favicon: version("favicon"),
    })
}

pub async fn asset(State(state): State<AppState>, Path(kind): Path<String>) -> AppResult<Response> {
    known(&kind)?;
    let (content_type, data): (String, Vec<u8>) =
        sqlx::query_as("SELECT content_type, data FROM brand_assets WHERE kind = $1")
            .bind(&kind)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    let mut response = data.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).map_err(|error| AppError::Internal(error.into()))?,
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(response)
}

pub async fn update(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<Branding>,
) -> AppResult<Json<BrandingInfo>> {
    let name = input
        .name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    if name
        .as_ref()
        .is_some_and(|name| name.chars().count() > MAX_NAME)
    {
        return Err(AppError::field(
            "name",
            format!("Gebruik hoogstens {MAX_NAME} tekens."),
        ));
    }
    let branding = Branding {
        name,
        accent: color(input.accent.as_deref(), "accent")?,
        accent_dark: color(input.accent_dark.as_deref(), "accent_dark")?,
        neutral: choice(input.neutral.as_deref(), &NEUTRALS, "neutral")?,
        radius: choice(input.radius.as_deref(), &RADII, "radius")?,
    };
    settings::set(&state.db, "branding", &branding).await?;
    Ok(Json(info(&state).await?))
}

pub async fn stylesheet(State(state): State<AppState>) -> AppResult<Response> {
    let theme: ThemeCss = settings::get(&state.db, "theme_css").await?;
    if theme.css.trim().is_empty() {
        return Err(AppError::NotFound);
    }
    let mut response = theme.css.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    Ok(response)
}

pub async fn update_css(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<ThemeCss>,
) -> AppResult<Json<BrandingInfo>> {
    if input.css.len() > MAX_CSS {
        return Err(AppError::field("css", "Gebruik hoogstens 64 kB aan CSS."));
    }
    if input.css.contains('\0') {
        return Err(AppError::field("css", "Deze CSS bevat ongeldige tekens."));
    }
    settings::set(&state.db, "theme_css", &ThemeCss { css: input.css }).await?;
    Ok(Json(info(&state).await?))
}

pub async fn upload(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(kind): Path<String>,
    Json(input): Json<Upload>,
) -> AppResult<Json<BrandingInfo>> {
    known(&kind)?;
    let encoded = input
        .data
        .split_once(";base64,")
        .map_or(input.data.as_str(), |(_, data)| data);
    let unreadable = || AppError::field("file", "Kies een PNG-, JPG- of WebP-afbeelding.");
    if encoded.len() > MAX_BYTES * 4 / 3 + 4 {
        return Err(AppError::field(
            "file",
            "Kies een bestand van hoogstens 512 kB.",
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|_| unreadable())?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::field(
            "file",
            "Kies een bestand van hoogstens 512 kB.",
        ));
    }
    let content_type = sniff(&bytes, &kind).ok_or_else(unreadable)?;
    sqlx::query(
        "INSERT INTO brand_assets (kind, content_type, data) VALUES ($1, $2, $3)
         ON CONFLICT (kind) DO UPDATE SET content_type = EXCLUDED.content_type, data = EXCLUDED.data, updated_at = now()",
    )
    .bind(&kind)
    .bind(content_type)
    .bind(bytes)
    .execute(&state.db)
    .await?;
    Ok(Json(info(&state).await?))
}

pub async fn remove(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(kind): Path<String>,
) -> AppResult<(StatusCode, Json<BrandingInfo>)> {
    known(&kind)?;
    sqlx::query("DELETE FROM brand_assets WHERE kind = $1")
        .bind(&kind)
        .execute(&state.db)
        .await?;
    Ok((StatusCode::OK, Json(info(&state).await?)))
}

#[cfg(test)]
mod tests {
    use super::{NEUTRALS, RADII, accent, choice, css_version, sniff};

    #[test]
    fn only_plain_images_are_accepted() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\nrest", "logo"), Some("image/png"));
        assert_eq!(sniff(b"<svg onload=alert(1)>", "logo"), None);
        assert_eq!(sniff(&[0, 0, 1, 0, 1], "logo"), None);
        assert_eq!(sniff(&[0, 0, 1, 0, 1], "favicon"), Some("image/x-icon"));
    }

    #[test]
    fn accents_are_six_digit_hex() {
        assert_eq!(accent("#4F46E5").as_deref(), Some("#4f46e5"));
        assert_eq!(accent("4f46e5").as_deref(), Some("#4f46e5"));
        assert_eq!(accent("red"), None);
        assert_eq!(accent("#fff"), None);
        assert_eq!(accent("#4f46e5;x"), None);
    }

    #[test]
    fn theme_choices_are_a_fixed_list() {
        assert_eq!(
            choice(Some("slate"), &NEUTRALS, "neutral")
                .unwrap()
                .as_deref(),
            Some("slate")
        );
        assert_eq!(choice(Some("zinc"), &NEUTRALS, "neutral").unwrap(), None);
        assert_eq!(choice(None, &RADII, "radius").unwrap(), None);
        assert!(choice(Some("red; }"), &NEUTRALS, "neutral").is_err());
        assert!(choice(Some("huge"), &RADII, "radius").is_err());
    }

    #[test]
    fn empty_css_has_no_version() {
        assert_eq!(css_version("  \n"), None);
        assert_eq!(css_version("a{}").map(|version| version.len()), Some(12));
    }
}
