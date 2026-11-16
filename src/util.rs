use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};

pub fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        return home_dir();
    }
    if let Some(rest) = input.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(input)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

pub fn now() -> DateTime<Local> {
    Local::now()
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn ext_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "text/plain" => "txt",
        _ => "bin",
    }
}

/// `photo.png` -> `photo-1.png`, `photo-2.png`, ...
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for n in 1..10_000 {
        let candidate = if ext.is_empty() {
            parent.join(format!("{stem}-{n}"))
        } else {
            parent.join(format!("{stem}-{n}.{ext}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

/// Expand the small template language used for filenames.
///
/// Supported: `{action}`, `{date}`, `{time}`, `{datetime}`, `{ext}`, `{unix}`.
pub fn expand_template(template: &str, action: &str, ext: &str) -> String {
    let stamp = now();
    let mut out = template.to_string();
    for (needle, value) in [
        ("{action}", action.to_string()),
        ("{date}", stamp.format("%Y-%m-%d").to_string()),
        ("{time}", stamp.format("%H-%M-%S").to_string()),
        ("{datetime}", stamp.format("%Y-%m-%d_%H-%M-%S").to_string()),
        ("{unix}", stamp.timestamp().to_string()),
        ("{ext}", ext.to_string()),
    ] {
        out = out.replace(needle, &value);
    }
    out
}

pub fn sanitize_component(input: &str) -> String {
    let mut cleaned: String = input
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    // A template must never be able to escape the save directory.
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", ".");
    }
    let trimmed = cleaned.trim_matches(|c| c == '-' || c == '.').to_string();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed
    }
}

/// Percent-encode for `application/x-www-form-urlencoded` bodies.
pub fn form_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let decoded = percent_decode(rest);
    Some(PathBuf::from(decoded))
}

pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(from, to)?;
    Ok(())
}

pub fn is_video_ext(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("mp4") | Some("webm") | Some("mkv") | Some("gif")
    )
}

pub fn truncate_text(input: &str, max: usize) -> String {
    let mut out: String = input.chars().take(max).collect();
    if input.chars().count() > max {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_expands_all_tokens() {
        let out = expand_template("{action}-{date}_{time}.{ext}", "region", "png");
        assert!(out.starts_with("region-"));
        assert!(out.ends_with(".png"));
        assert!(!out.contains('{'));
    }

    #[test]
    fn form_encoding_matches_html_rules() {
        assert_eq!(form_encode("a b&c=d/e"), "a+b%26c%3Dd%2Fe");
        assert_eq!(form_encode("safe-._~"), "safe-._~");
    }

    #[test]
    fn tile_uri_round_trips() {
        let path = file_uri_to_path("file:///tmp/a%20b.png").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/a b.png"));
    }

    #[test]
    fn sanitize_strips_separators() {
        assert_eq!(sanitize_component("../../etc/passwd"), "etc-passwd");
        assert_eq!(sanitize_component("///"), "file");
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(human_bytes(999), "999 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");
    }
}
