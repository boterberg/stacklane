use std::fmt::Write;

pub const PAGE_WIDTH: f32 = 595.0;
pub const PAGE_HEIGHT: f32 = 842.0;

#[derive(Clone, Copy)]
pub enum Font {
    Regular,
    Bold,
}

impl Font {
    fn resource(self) -> &'static str {
        match self {
            Self::Regular => "F1",
            Self::Bold => "F2",
        }
    }
}

const HELVETICA_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

fn encode(text: &str) -> Vec<u8> {
    text.chars()
        .map(|character| match character {
            '€' => 0x80,
            '‘' | '’' => b'\'',
            '“' | '”' => b'"',
            '–' | '—' => b'-',
            '…' => b'.',
            character if (character as u32) < 0x100 => character as u32 as u8,
            _ => b'?',
        })
        .collect()
}

pub fn text_width(text: &str, size: f32, font: Font) -> f32 {
    let units: f32 = encode(text)
        .iter()
        .map(|byte| match byte {
            32..=126 => f32::from(HELVETICA_WIDTHS[usize::from(byte - 32)]),
            0x80 => 556.0,
            _ => 556.0,
        })
        .sum();
    let bold = match font {
        Font::Regular => 1.0,
        Font::Bold => 1.04,
    };
    units * size / 1000.0 * bold
}

fn escape(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        match byte {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(byte as char);
            }
            32..=126 => out.push(byte as char),
            _ => {
                let _ = write!(out, "\\{byte:03o}");
            }
        }
    }
    out
}

#[derive(Default)]
pub struct Page {
    content: String,
}

impl Page {
    pub fn text(&mut self, x: f32, y: f32, size: f32, font: Font, gray: f32, text: &str) {
        let _ = writeln!(
            self.content,
            "BT {gray:.2} g /{} {size:.1} Tf {x:.2} {:.2} Td ({}) Tj ET",
            font.resource(),
            PAGE_HEIGHT - y,
            escape(&encode(text))
        );
    }

    pub fn text_right(&mut self, right: f32, y: f32, size: f32, font: Font, gray: f32, text: &str) {
        self.text(
            right - text_width(text, size, font),
            y,
            size,
            font,
            gray,
            text,
        );
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, gray: f32) {
        let _ = writeln!(
            self.content,
            "{gray:.2} G 0.6 w {x1:.2} {:.2} m {x2:.2} {:.2} l S",
            PAGE_HEIGHT - y1,
            PAGE_HEIGHT - y2
        );
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32, gray: f32) {
        let _ = writeln!(
            self.content,
            "{gray:.2} g {x:.2} {:.2} {width:.2} {height:.2} re f",
            PAGE_HEIGHT - y - height
        );
    }
}

pub fn wrap(text: &str, width: f32, size: f32, font: Font) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if text_width(&candidate, size, font) > width && !current.is_empty() {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub fn render(pages: &[Page], title: &str) -> Vec<u8> {
    let first_page = 6;
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let kids: Vec<String> = (0..pages.len())
        .map(|index| format!("{} 0 R", first_page + index * 2))
        .collect();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
    );
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    objects.push(
        format!(
            "<< /Title ({}) /Producer (Stacklane) >>",
            escape(&encode(title))
        )
        .into_bytes(),
    );
    for (index, page) in pages.iter().enumerate() {
        let content_id = first_page + index * 2 + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", page.content.len()).into_bytes();
        stream.extend_from_slice(page.content.as_bytes());
        stream.extend_from_slice(b"endstream");
        objects.push(stream);
    }

    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_valid_looking_document() {
        let mut page = Page::default();
        page.text(
            40.0,
            60.0,
            12.0,
            Font::Bold,
            0.0,
            "Factuur (€ 6,50) – België",
        );
        page.line(40.0, 70.0, 555.0, 70.0, 0.8);
        let pdf = render(&[page], "Factuur 100001");
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/BaseFont /Helvetica-Bold"));
        assert!(text.contains("\\(\\200 6,50\\)"));
        assert!(text.contains("Belgi\\353"));
        assert!(text.trim_end().ends_with("%%EOF"));
    }

    #[test]
    fn wraps_long_text() {
        let lines = wrap(
            "een twee drie vier vijf zes zeven acht negen tien",
            60.0,
            10.0,
            Font::Regular,
        );
        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|line| text_width(line, 10.0, Font::Regular) <= 60.0 || !line.contains(' '))
        );
    }
}
