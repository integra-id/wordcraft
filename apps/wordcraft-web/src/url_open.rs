//! `?url=` / `?src=`: open a document from an address at start-up (`docs/viewer.md`). The address
//! is resolved against the page, fetched (cookies only for the page's own origin) and the bytes go
//! through the inbox as a dropped file does. These are the parts that don't need a browser.

/// What a document is called when its address names nothing usable.
pub const FALLBACK_NAME: &str = "document.docx";
/// Longest document name kept from `?name=` or the address.
const MAX_NAME_CHARS: usize = 200;

/// The address to open: `?url=`, else its alias `?src=`; blank values count as missing.
pub fn requested(url: Option<String>, src: Option<String>) -> Option<String> {
    [url, src].into_iter().flatten().map(|s| s.trim().to_string()).find(|s| !s.is_empty())
}

/// Whether the viewer may fetch an address with this scheme (`https`, `http:` or `HTTPS`, as
/// `URL.protocol` gives it). Only web addresses and the page's own blobs: `javascript:` never,
/// and `data:` isn't worth the risk of very long addresses.
pub fn check_scheme(scheme: &str) -> Result<(), String> {
    let scheme = scheme.trim().trim_end_matches(':').to_ascii_lowercase();
    match scheme.as_str() {
        "http" | "https" | "blob" => Ok(()),
        "" => Err("the address has no scheme".into()),
        other => Err(format!("{other}: addresses can't be opened; use http, https or blob")),
    }
}

/// The document's name: `?name=` if given, else the address's last path segment (query and
/// fragment dropped, percent-decoded), else [`FALLBACK_NAME`]. Only a file name is kept.
pub fn doc_name(name: Option<&str>, address: &str) -> String {
    if let Some(n) = name.map(file_part).filter(|n| !n.is_empty()) {
        return n;
    }
    if is_opaque(address) {
        // javascript:…, data:…, mailto:…: nothing that reads as a file name.
        return FALLBACK_NAME.into();
    }
    let path = address.split(['?', '#']).next().unwrap_or("");
    let last = path.rsplit('/').next().unwrap_or("");
    let n = file_part(&percent_decode(last));
    if n.is_empty() { FALLBACK_NAME.into() } else { n }
}

/// `scheme:` not followed by `/`, such as `data:text/plain,…`; one-letter schemes are drive
/// letters (`C:\…`) and `blob:https://…` counts as a path.
fn is_opaque(address: &str) -> bool {
    let Some((scheme, rest)) = address.split_once(':') else { return false };
    let valid = scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid && !rest.starts_with('/') && !scheme.eq_ignore_ascii_case("blob")
}

/// The part after the last `/` or `\`, without control characters, trimmed and capped.
fn file_part(s: &str) -> String {
    let last = s.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = last.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
    let clean = clean.trim();
    if clean == "." || clean == ".." { String::new() } else { clean.to_string() }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && let (Some(hi), Some(lo)) = (b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex))
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_wins_over_src_and_blanks_are_missing() {
        assert_eq!(requested(Some("a.docx".into()), Some("b.docx".into())).as_deref(), Some("a.docx"));
        assert_eq!(requested(None, Some(" b.docx ".into())).as_deref(), Some("b.docx"));
        assert_eq!(requested(Some("  ".into()), Some("b.docx".into())).as_deref(), Some("b.docx"));
        assert_eq!(requested(Some(String::new()), None), None);
        assert_eq!(requested(None, None), None);
    }

    #[test]
    fn only_web_and_blob_schemes() {
        for ok in ["http:", "https:", "HTTPS:", "blob:", "https"] {
            assert!(check_scheme(ok).is_ok(), "{ok}");
        }
        for bad in ["javascript:", "JavaScript:", "data:", "file:", "ftp:", "about:", "chrome:", ""] {
            assert!(check_scheme(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn name_from_the_address() {
        assert_eq!(doc_name(None, "https://example.com/files/report.docx"), "report.docx");
        assert_eq!(doc_name(None, "https://example.com/a/Q3%20plan%20%C3%A9t%C3%A9.odt?token=1#p2"), "Q3 plan été.odt");
        assert_eq!(doc_name(None, "/test.docx"), "test.docx");
        assert_eq!(doc_name(None, "https://example.com/download?id=5"), "download");
        assert_eq!(doc_name(None, "https://example.com/"), FALLBACK_NAME);
        assert_eq!(doc_name(None, "https://example.com"), "example.com");
        assert_eq!(doc_name(None, "https://example.com/x/..%2F..%2Fevil.rtf"), "evil.rtf");
        assert_eq!(doc_name(None, "https://example.com/%2E%2E"), FALLBACK_NAME);
        assert_eq!(doc_name(None, "https://example.com/bad%zz.md"), "bad%zz.md");
        assert_eq!(doc_name(None, "blob:https://example.com/5b1c-77aa"), "5b1c-77aa");
        assert_eq!(doc_name(None, "javascript:alert(1)"), FALLBACK_NAME);
        assert_eq!(doc_name(None, "data:application/octet-stream;base64,AAAA"), FALLBACK_NAME);
        assert_eq!(doc_name(None, "C:\\docs\\a.docx"), "a.docx");
    }

    #[test]
    fn name_param_wins_and_is_only_a_file_name() {
        assert_eq!(doc_name(Some("Minutes.docx"), "https://example.com/x/y.rtf"), "Minutes.docx");
        assert_eq!(doc_name(Some("../../etc/passwd"), "https://example.com/y.rtf"), "passwd");
        assert_eq!(doc_name(Some("C:\\dir\\a.doc"), "https://example.com/y.rtf"), "a.doc");
        assert_eq!(doc_name(Some("  "), "https://example.com/y.rtf"), "y.rtf");
        assert_eq!(doc_name(Some("a\u{0}b\n.txt"), "/y.rtf"), "ab.txt");
        assert_eq!(doc_name(Some(&"x".repeat(500)), "/y.rtf").chars().count(), MAX_NAME_CHARS);
    }
}
