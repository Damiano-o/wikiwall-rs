//! Small helpers shared by the rest of the crate.

pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Decode `%XX` sequences, leaving invalid escapes untouched.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(value) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encode a title for use in a Wikipedia URL path.
pub fn percent_encode_path(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub fn hex_color(value: &str) -> (u8, u8, u8) {
    let cleaned = value.trim().trim_start_matches('#');
    if cleaned.len() < 6 {
        return (0, 0, 0);
    }
    let parse = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&cleaned[range], 16).unwrap_or(0)
    };
    (parse(0..2), parse(2..4), parse(4..6))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup() {
        assert_eq!(html_escape("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
    }

    #[test]
    fn decodes_percent_sequences() {
        assert_eq!(percent_decode("Radioso_maggio%21"), "Radioso_maggio!");
        assert_eq!(percent_decode("no%zz"), "no%zz");
    }

    #[test]
    fn reads_hex_colors() {
        assert_eq!(hex_color("#970302"), (0x97, 0x03, 0x02));
        assert_eq!(hex_color("fbf7ef"), (0xfb, 0xf7, 0xef));
    }
}
