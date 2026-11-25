use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentUser, Staff, permission};
use crate::error::{AppError, AppResult};
use crate::html::{sanitize_rich_text, visible_text};
use crate::pagination::like_pattern;
use crate::state::AppState;

const MAX_TITLE: usize = 150;
const MAX_CATEGORY: usize = 60;
const MAX_BODY: usize = 200_000;

#[derive(Deserialize)]
pub struct Search {
    search: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ArticleSummary {
    id: Uuid,
    slug: String,
    title: String,
    category: String,
    excerpt: String,
    published: bool,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Article {
    id: Uuid,
    slug: String,
    title: String,
    category: String,
    body_html: String,
    published: bool,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct ArticleInput {
    title: String,
    #[serde(default)]
    category: String,
    body_html: String,
    #[serde(default)]
    published: bool,
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for character in title.to_lowercase().chars() {
        let character = match character {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            other => other,
        };
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(80).collect();
    if slug.is_empty() {
        "artikel".to_owned()
    } else {
        slug.trim_end_matches('-').to_owned()
    }
}

fn excerpt(html: &str) -> String {
    let text = visible_text(html)
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"");
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() > 160 {
        format!(
            "{}…",
            collapsed.chars().take(159).collect::<String>().trim_end()
        )
    } else {
        collapsed
    }
}

fn clean(input: ArticleInput) -> AppResult<(String, String, String, bool)> {
    let mut errors = std::collections::BTreeMap::new();
    let title = input.title.trim().to_owned();
    let category = input.category.trim().to_owned();
    if title.chars().count() < 3 {
        errors.insert(
            "title",
            "Geef het artikel een titel van minstens 3 tekens.".to_owned(),
        );
    } else if title.chars().count() > MAX_TITLE {
        errors.insert("title", format!("Gebruik hoogstens {MAX_TITLE} tekens."));
    }
    if category.chars().count() > MAX_CATEGORY {
        errors.insert(
            "category",
            format!("Gebruik hoogstens {MAX_CATEGORY} tekens."),
        );
    }
    let body = sanitize_rich_text(&input.body_html);
    if visible_text(&body).is_empty() {
        errors.insert("body_html", "Schrijf de tekst van het artikel.".to_owned());
    } else if body.len() > MAX_BODY {
        errors.insert("body_html", "Dit artikel is te lang.".to_owned());
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    Ok((title, category, body, input.published))
}

async fn free_slug(state: &AppState, title: &str, own: Option<Uuid>) -> AppResult<String> {
    let base = slugify(title);
    for attempt in 1..100 {
        let slug = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM kb_articles WHERE slug = $1 AND ($2::uuid IS NULL OR id <> $2))",
        )
        .bind(&slug)
        .bind(own)
        .fetch_one(&state.db)
        .await?;
        if !taken {
            return Ok(slug);
        }
    }
    Ok(format!("{base}-{}", Uuid::new_v4().simple()))
}

async fn list(
    state: &AppState,
    search: Option<&str>,
    drafts: bool,
) -> AppResult<Vec<ArticleSummary>> {
    let rows: Vec<Article> = sqlx::query_as(
        "SELECT id, slug, title, category, body_html, published, updated_at FROM kb_articles
         WHERE ($1 OR published) AND ($2::text IS NULL OR title ILIKE $2 OR body_html ILIKE $2 OR category ILIKE $2)
         ORDER BY lower(category), lower(title)
         LIMIT 500",
    )
    .bind(drafts)
    .bind(like_pattern(search))
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ArticleSummary {
            excerpt: excerpt(&row.body_html),
            id: row.id,
            slug: row.slug,
            title: row.title,
            category: row.category,
            published: row.published,
            updated_at: row.updated_at,
        })
        .collect())
}

pub async fn index(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(query): Query<Search>,
) -> AppResult<Json<Vec<ArticleSummary>>> {
    Ok(Json(list(&state, query.search.as_deref(), false).await?))
}

pub async fn show(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(slug): Path<String>,
) -> AppResult<Json<Article>> {
    let article = sqlx::query_as(
        "SELECT id, slug, title, category, body_html, published, updated_at FROM kb_articles WHERE slug = $1 AND published",
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(article))
}

pub async fn admin_index(
    State(state): State<AppState>,
    _admin: Staff<permission::Support>,
    Query(query): Query<Search>,
) -> AppResult<Json<Vec<ArticleSummary>>> {
    Ok(Json(list(&state, query.search.as_deref(), true).await?))
}

pub async fn admin_show(
    State(state): State<AppState>,
    _admin: Staff<permission::Support>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Article>> {
    let article = sqlx::query_as(
        "SELECT id, slug, title, category, body_html, published, updated_at FROM kb_articles WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(article))
}

pub async fn create(
    State(state): State<AppState>,
    _admin: Staff<permission::Support>,
    Json(input): Json<ArticleInput>,
) -> AppResult<(StatusCode, Json<Article>)> {
    let (title, category, body, published) = clean(input)?;
    let slug = free_slug(&state, &title, None).await?;
    let article = sqlx::query_as(
        "INSERT INTO kb_articles (id, slug, title, category, body_html, published) VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, slug, title, category, body_html, published, updated_at",
    )
    .bind(Uuid::new_v4())
    .bind(slug)
    .bind(title)
    .bind(category)
    .bind(body)
    .bind(published)
    .fetch_one(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(article)))
}

pub async fn update(
    State(state): State<AppState>,
    _admin: Staff<permission::Support>,
    Path(id): Path<Uuid>,
    Json(input): Json<ArticleInput>,
) -> AppResult<Json<Article>> {
    let (title, category, body, published) = clean(input)?;
    let slug = free_slug(&state, &title, Some(id)).await?;
    let article = sqlx::query_as(
        "UPDATE kb_articles SET slug = $2, title = $3, category = $4, body_html = $5, published = $6, updated_at = now()
         WHERE id = $1 RETURNING id, slug, title, category, body_html, published, updated_at",
    )
    .bind(id)
    .bind(slug)
    .bind(title)
    .bind(category)
    .bind(body)
    .bind(published)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(article))
}

pub async fn remove(
    State(state): State<AppState>,
    _admin: Staff<permission::Support>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted = sqlx::query("DELETE FROM kb_articles WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{excerpt, slugify};

    #[test]
    fn slugs_are_readable() {
        assert_eq!(slugify("Hoe stel ik DNS in?"), "hoe-stel-ik-dns-in");
        assert_eq!(slugify("Één café & méér"), "een-cafe-meer");
        assert_eq!(slugify("!!!"), "artikel");
    }

    #[test]
    fn excerpts_are_short_plain_text() {
        assert_eq!(
            excerpt("<p>Hallo &amp; <strong>welkom</strong></p>"),
            "Hallo & welkom"
        );
        assert!(excerpt(&format!("<p>{}</p>", "woord ".repeat(100))).ends_with('…'));
    }
}
