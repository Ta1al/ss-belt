use anyhow::Result;
use serde::Deserialize;
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::user;
use crate::error::HintExt;

/// A ShareX `.sxcu` custom uploader definition.
///
/// Only the fields that matter for performing an upload are typed; unknown keys
/// (ShareX writes plenty) are ignored so real-world files load unchanged.
#[derive(Debug, Clone, Deserialize)]
pub struct Sxcu {
    #[serde(rename = "Version", default)]
    pub version: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "DestinationType", default)]
    pub destination_type: String,
    #[serde(rename = "RequestMethod", default)]
    pub request_method: String,
    #[serde(rename = "RequestURL", default)]
    pub request_url: String,
    #[serde(rename = "Headers", default)]
    pub headers: BTreeMap<String, String>,
    #[serde(rename = "Body", default)]
    pub body: String,
    #[serde(rename = "Arguments", default)]
    pub arguments: BTreeMap<String, String>,
    #[serde(rename = "FileFormName", default)]
    pub file_form_name: String,
    #[serde(rename = "URL", default)]
    pub url: String,
    #[serde(rename = "ThumbnailURL", default)]
    pub thumbnail_url: String,
    #[serde(rename = "DeletionURL", default)]
    pub deletion_url: String,
    #[serde(rename = "ErrorMessage", default)]
    pub error_message: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Multipart,
    Form,
    Json,
    Xml,
    Binary,
}

impl BodyKind {
    pub fn parse(input: &str) -> Result<Self> {
        match input.trim() {
            "" | "MultipartFormData" => Ok(BodyKind::Multipart),
            "FormURLEncoded" => Ok(BodyKind::Form),
            "JSON" => Ok(BodyKind::Json),
            "XML" => Ok(BodyKind::Xml),
            "Binary" => Ok(BodyKind::Binary),
            other => Err(user(format!("unsupported .sxcu Body type `{other}`"))),
        }
    }
}

impl Sxcu {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|err| user(format!("cannot read uploader {}: {err}", path.display())))?;
        Self::parse(&text)
            .map_err(|err| user(format!("invalid uploader {}: {err}", path.display())))
    }

    pub fn parse(text: &str) -> Result<Self> {
        let sxcu: Sxcu = serde_json::from_str(text)?;
        if sxcu.request_url.trim().is_empty() {
            return Err(user(".sxcu has no RequestURL"));
        }
        if !sxcu.request_url.starts_with("http://") && !sxcu.request_url.starts_with("https://") {
            return Err(user(".sxcu RequestURL must be http(s)"));
        }
        Ok(sxcu)
    }

    pub fn method(&self) -> String {
        let method = self.request_method.trim().to_ascii_uppercase();
        if method.is_empty() { "POST".to_string() } else { method }
    }

    pub fn body_kind(&self) -> Result<BodyKind> {
        BodyKind::parse(&self.body)
    }
}

pub struct Vars<'a> {
    pub filename: &'a str,
    pub mime: &'a str,
    pub response: &'a str,
}

/// Expand the subset of ShareX template variables that can be resolved without
/// a regex engine: file names, the raw response and JSON paths.
pub fn expand(template: &str, vars: &Vars<'_>) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'$' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let Some(end) = template[i + 1..].find('$').map(|offset| i + 1 + offset) else {
            out.push('$');
            i += 1;
            continue;
        };
        let token = &template[i + 1..end];
        let replacement = match token {
            "filename" | "input" => vars.filename.to_string(),
            "response" => vars.response.to_string(),
            "mime" => vars.mime.to_string(),
            token if token.starts_with("json:") => {
                let path = &token[5..];
                let json: Json = serde_json::from_str(vars.response)
                    .map_err(|_| user(format!("upload response is not JSON (wanted ${token}$)")))?;
                json_lookup(&json, path)
                    .map(|value| match value {
                        Json::String(text) => text.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            }
            token if token.starts_with("regex:") => {
                return Err(user(
                    "$regex: templates need regular expressions, which this version does not support",
                )
                .hint("edit the .sxcu to use $json: or $response$ instead"))
            }
            token if token.starts_with("xml:") => {
                return Err(user("$xml: templates are not supported yet")
                    .hint("use a JSON-based uploader instead"))
            }
            _ => String::new(),
        };
        out.push_str(&replacement);
        i = end + 1;
    }
    Ok(out)
}

fn json_lookup<'a>(value: &'a Json, path: &str) -> Option<&'a Json> {
    let mut current = value;
    for segment in path.split('.').filter(|s| !s.is_empty()) {
        let (name, index) = match segment.find('[') {
            Some(open) if segment.ends_with(']') => {
                let idx: usize = segment[open + 1..segment.len() - 1].parse().ok()?;
                (&segment[..open], Some(idx))
            }
            _ => (segment, None),
        };
        if !name.is_empty() {
            current = current.get(name)?;
        }
        if let Some(index) = index {
            current = current.get(index)?;
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "Version": "13.7.0",
        "Name": "Example",
        "DestinationType": "ImageUploader",
        "RequestMethod": "POST",
        "RequestURL": "https://example.com/upload",
        "Headers": {"Authorization": "Bearer t"},
        "Body": "MultipartFormData",
        "Arguments": {"album": "shots"},
        "FileFormName": "file",
        "URL": "$json:data.url$",
        "DeletionURL": "$json:data.delete$"
    }"#;

    #[test]
    fn parses_a_realistic_sxcu() {
        let sxcu = Sxcu::parse(SAMPLE).unwrap();
        assert_eq!(sxcu.name, "Example");
        assert_eq!(sxcu.method(), "POST");
        assert_eq!(sxcu.body_kind().unwrap(), BodyKind::Multipart);
        assert_eq!(sxcu.file_form_name, "file");
    }

    #[test]
    fn json_paths_support_indexes() {
        let response = r#"{"files":[{"url":"https://x/y.png"}]}"#;
        let vars = Vars { filename: "a.png", mime: "image/png", response };
        assert_eq!(expand("$json:files[0].url$", &vars).unwrap(), "https://x/y.png");
        assert_eq!(expand("$json:files[0].missing$", &vars).unwrap(), "");
    }

    #[test]
    fn file_and_response_tokens_expand() {
        let vars = Vars { filename: "shot.png", mime: "image/png", response: "ok" };
        assert_eq!(
            expand("https://h/$filename$?r=$response$", &vars).unwrap(),
            "https://h/shot.png?r=ok"
        );
    }

    #[test]
    fn unsupported_tokens_are_explicit_errors() {
        let vars = Vars { filename: "a", mime: "text/plain", response: "x" };
        let err = expand("$regex:(\\d+)$", &vars).unwrap_err();
        assert!(err.to_string().contains("regex"));
    }

    #[test]
    fn missing_request_url_is_rejected() {
        assert!(Sxcu::parse(r#"{"Name":"x"}"#).is_err());
    }

    #[test]
    fn unknown_body_type_is_reported() {
        let err = BodyKind::parse("GraphQL").unwrap_err();
        assert!(err.to_string().contains("GraphQL"));
    }
}
