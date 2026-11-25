use std::collections::HashSet;

pub fn sanitize_rich_text(input: &str) -> String {
    let tags: HashSet<&str> = [
        "p",
        "br",
        "strong",
        "em",
        "s",
        "u",
        "mark",
        "code",
        "pre",
        "blockquote",
        "ul",
        "ol",
        "li",
        "h1",
        "h2",
        "h3",
        "a",
        "hr",
    ]
    .into_iter()
    .collect();
    ammonia::Builder::default()
        .tags(tags)
        .link_rel(Some("noopener noreferrer nofollow"))
        .url_schemes(["http", "https", "mailto"].into_iter().collect())
        .clean(input)
        .to_string()
}

pub fn visible_text(html: &str) -> String {
    ammonia::Builder::empty()
        .clean(html)
        .to_string()
        .replace("&nbsp;", " ")
        .trim()
        .to_owned()
}

pub fn plain_text(html: &str) -> String {
    let spaced = html
        .replace("<br>", "\n")
        .replace("</p>", "</p>\n\n")
        .replace("<li>", "<li>- ")
        .replace("</li>", "</li>\n")
        .replace("</ul>", "</ul>\n")
        .replace("</ol>", "</ol>\n")
        .replace("</blockquote>", "</blockquote>\n\n")
        .replace("</pre>", "</pre>\n\n");
    let text = ammonia::Builder::empty()
        .clean(&spaced)
        .to_string()
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim_end) {
        if line.is_empty() && lines.last().is_none_or(|last| last.is_empty()) {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scripts_and_event_handlers() {
        let cleaned = sanitize_rich_text(
            r#"<p onclick="x()">Hoi<script>alert(1)</script></p><img src=x onerror=y>"#,
        );
        assert_eq!(cleaned, "<p>Hoi</p>");
    }

    #[test]
    fn keeps_formatting_and_safe_links() {
        let cleaned = sanitize_rich_text(
            r#"<p><strong>a</strong> <a href="https://x.nl">b</a> <a href="javascript:alert(1)">c</a></p>"#,
        );
        assert_eq!(
            cleaned,
            r#"<p><strong>a</strong> <a href="https://x.nl" rel="noopener noreferrer nofollow">b</a> <a rel="noopener noreferrer nofollow">c</a></p>"#
        );
    }

    #[test]
    fn empty_paragraphs_have_no_visible_text() {
        assert_eq!(visible_text("<p></p><p> </p><br>"), "");
        assert_eq!(visible_text("<p>Hallo</p>"), "Hallo");
    }

    #[test]
    fn plain_text_keeps_paragraphs_and_lists() {
        assert_eq!(
            plain_text(
                "<p>Hoi &amp; welkom</p><p></p><ul><li>een</li><li>twee</li></ul><p>a<br>b &lt;c&gt;</p>"
            ),
            "Hoi & welkom\n\n- een\n- twee\n\na\nb <c>"
        );
    }
}
