use crate::html::sanitize_rich_text;

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn plain_to_html(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    let html: String = normalized
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(|paragraph| format!("<p>{}</p>", escape(paragraph).replace('\n', "<br>")))
        .collect();
    let cleaned = sanitize_rich_text(&html);
    if cleaned.is_empty() {
        "<p>(leeg bericht)</p>".to_owned()
    } else {
        cleaned
    }
}

pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for character in name.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::plain_to_html;

    #[test]
    fn plain_text_becomes_safe_paragraphs() {
        assert_eq!(
            plain_to_html("Hallo,\r\n\r\nRegel een\r\nregel <b>twee</b>"),
            "<p>Hallo,</p><p>Regel een<br>regel &lt;b&gt;twee&lt;/b&gt;</p>"
        );
        assert_eq!(plain_to_html("  "), "<p>(leeg bericht)</p>");
    }
}
