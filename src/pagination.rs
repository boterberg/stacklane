use serde::{Deserialize, Serialize};

const ALLOWED_PAGE_SIZES: [i64; 4] = [5, 10, 25, 50];

pub fn like_pattern(search: Option<&str>) -> Option<String> {
    let search = search.map(str::trim).filter(|search| !search.is_empty())?;
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    Some(format!("%{escaped}%"))
}

#[derive(Deserialize)]
pub struct PageQuery {
    page: Option<String>,
    per_page: Option<String>,
}

impl PageQuery {
    pub fn page(&self) -> i64 {
        self.page
            .as_deref()
            .and_then(|page| page.parse().ok())
            .unwrap_or(1i64)
            .clamp(1, 100_000)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page
            .as_deref()
            .and_then(|size| size.parse().ok())
            .filter(|size| ALLOWED_PAGE_SIZES.contains(size))
            .unwrap_or(10)
    }

    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.per_page()
    }
}

#[derive(Serialize)]
pub struct PageMeta {
    total: i64,
    page: i64,
    per_page: i64,
    from: i64,
    to: i64,
}

#[derive(Serialize)]
pub struct Paginated<T> {
    data: Vec<T>,
    meta: PageMeta,
}

impl<T> Paginated<T> {
    pub fn new(data: Vec<T>, total: i64, query: &PageQuery) -> Self {
        let from = if data.is_empty() {
            0
        } else {
            query.offset() + 1
        };
        let to = query.offset() + data.len() as i64;
        Self {
            data,
            meta: PageMeta {
                total,
                page: query.page(),
                per_page: query.per_page(),
                from,
                to,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PageQuery, like_pattern};

    #[test]
    fn a_huge_page_number_stays_a_valid_offset() {
        let query = PageQuery {
            page: Some(i64::MAX.to_string()),
            per_page: Some("50".into()),
        };
        assert!(query.offset() > 0);
    }

    #[test]
    fn search_wildcards_are_taken_literally() {
        assert_eq!(
            like_pattern(Some(" 50%_off ")).as_deref(),
            Some("%50\\%\\_off%")
        );
        assert_eq!(like_pattern(Some("  ")), None);
    }
}
