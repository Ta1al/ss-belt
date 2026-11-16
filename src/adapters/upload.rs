use anyhow::{Context, Result};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Duration;

use crate::config::Destination;
use crate::error::user;
use crate::sxcu::{self, Sxcu, Vars};
use crate::steps::{Ctx, Probe, Support, Value};
use crate::util;
use crate::error::HintExt;

pub fn support(probe: &Probe<'_>) -> Support {
    if probe.config.upload.destinations.is_empty() {
        Support::Degraded(
            "no upload destination configured; add one under [upload.destinations]".into(),
        )
    } else if probe.config.upload.default_destination.is_empty() {
        Support::Degraded("upload.destinations exist but upload.default_destination is empty".into())
    } else {
        Support::Ready
    }
}

struct Payload {
    filename: String,
    mime: String,
    bytes: Vec<u8>,
    is_text: bool,
}

impl Payload {
    fn from_ctx(ctx: &Ctx) -> Result<Self> {
        match ctx.value.clone() {
            Value::Image(path) | Value::Video(path) => {
                let bytes = fs::read(&path)
                    .with_context(|| format!("reading {} for upload", path.display()))?;
                Ok(Payload {
                    filename: path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "upload.bin".into()),
                    mime: util::mime_for_path(&path).to_string(),
                    bytes,
                    is_text: false,
                })
            }
            Value::Text(text) | Value::Url(text) => {
                let bytes = text.clone().into_bytes();
                Ok(Payload {
                    filename: format!("{}.txt", ctx.action),
                    mime: "text/plain".to_string(),
                    bytes,
                    is_text: true,
                })
            }
            Value::Empty => Err(user("nothing to upload")),
        }
    }
}

pub fn run(ctx: &mut Ctx) -> Result<()> {
    let mut payload = Payload::from_ctx(ctx)?;

    let destination_name = ctx
        .upload_destination
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| ctx.config.upload.default_destination.clone());
    if destination_name.is_empty() {
        return Err(user("no upload destination configured").hint(
            "set upload.default_destination and define it under [upload.destinations]",
        ));
    }

    let destination = ctx
        .config
        .upload
        .destinations
        .get(&destination_name)
        .cloned()
        .ok_or_else(|| user(format!("unknown upload destination `{destination_name}`")))?;

    if ctx.config.upload.strip_metadata && !payload.is_text {
        payload = strip_metadata(payload);
    }

    if ctx.dry_run {
        ctx.note(format!(
            "dry-run: would upload {} ({}) to `{destination_name}`",
            payload.filename,
            util::human_bytes(payload.bytes.len() as u64)
        ));
        ctx.value = Value::Url("https://example.invalid/dry-run".into());
        return Ok(());
    }

    let url = match destination {
        Destination::Sxcu { file } => upload_sxcu(ctx, &destination_name, &file, &payload)?,
        Destination::Presigned { url, method } => {
            upload_presigned(ctx, &url, &method, &payload)?
        }
    };

    ctx.note(format!("uploaded to {url}"));
    ctx.value = Value::Url(url);
    Ok(())
}

fn upload_sxcu(ctx: &Ctx, name: &str, file: &str, payload: &Payload) -> Result<String> {
    let path = util::expand_tilde(file);
    if !path.exists() {
        return Err(user(format!(
            "upload destination `{name}` points at {} which does not exist",
            path.display()
        )));
    }
    let sxcu = Sxcu::load(&path)?;
    let vars = Vars { filename: &payload.filename, mime: &payload.mime, response: "" };

    let mut headers: Vec<(String, String)> = Vec::new();
    for (key, value) in &sxcu.headers {
        headers.push((key.clone(), sxcu::expand(value, &vars)?));
    }

    let (body, content_type) = match sxcu.body_kind()? {
        sxcu::BodyKind::Multipart => {
            let boundary = format!("----ssbelt{}", chrono::Local::now().timestamp_nanos_opt().unwrap_or(0));
            let mut arguments = Vec::new();
            for (key, value) in &sxcu.arguments {
                arguments.push((key.clone(), sxcu::expand(value, &vars)?));
            }
            let field = if sxcu.file_form_name.is_empty() { "file" } else { &sxcu.file_form_name };
            let body = build_multipart(&boundary, field, payload, &arguments);
            (body, format!("multipart/form-data; boundary={boundary}"))
        }
        sxcu::BodyKind::Binary => (payload.bytes.clone(), payload.mime.clone()),
        sxcu::BodyKind::Form => {
            let mut pairs: Vec<(String, String)> = Vec::new();
            for (key, value) in &sxcu.arguments {
                pairs.push((key.clone(), sxcu::expand(value, &vars)?));
            }
            if payload.is_text && !sxcu.file_form_name.is_empty() {
                pairs.push((
                    sxcu.file_form_name.clone(),
                    String::from_utf8_lossy(&payload.bytes).to_string(),
                ));
            }
            let encoded = pairs
                .iter()
                .map(|(k, v)| format!("{}={}", util::form_encode(k), util::form_encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            (encoded.into_bytes(), "application/x-www-form-urlencoded".to_string())
        }
        sxcu::BodyKind::Json => {
            if !payload.is_text {
                return Err(user(
                    "this .sxcu uses a JSON body with a file, which is not supported yet",
                )
                .hint("use a multipart .sxcu uploader instead"));
            }
            let mut object = serde_json::Map::new();
            for (key, value) in &sxcu.arguments {
                object.insert(key.clone(), serde_json::Value::String(sxcu::expand(value, &vars)?));
            }
            if !sxcu.file_form_name.is_empty() {
                object.insert(
                    sxcu.file_form_name.clone(),
                    serde_json::Value::String(
                        String::from_utf8_lossy(&payload.bytes).to_string(),
                    ),
                );
            }
            (serde_json::to_vec(&object)?, "application/json".to_string())
        }
        sxcu::BodyKind::Xml => {
            return Err(user("XML bodies are not supported yet")
                .hint("use a multipart or JSON uploader"))
        }
    };

    headers.push(("Content-Type".to_string(), content_type));
    if !headers.iter().any(|(key, _)| key.eq_ignore_ascii_case("user-agent")) {
        headers.push(("User-Agent".to_string(), format!("ss-belt/{}", crate::VERSION)));
    }

    let (status, text) = http_send(
        &sxcu.method(),
        &sxcu.request_url,
        &headers,
        body,
        Duration::from_secs(ctx.config.upload.timeout_secs),
    )?;

    let vars = Vars { filename: &payload.filename, mime: &payload.mime, response: &text };

    if !(200..300).contains(&status) {
        let message = if sxcu.error_message.is_empty() {
            format!("upload failed with HTTP {status}: {}", util::truncate_text(&text, 300))
        } else {
            format!(
                "upload failed with HTTP {status}: {}",
                sxcu::expand(&sxcu.error_message, &vars).unwrap_or_else(|_| text.clone())
            )
        };
        return Err(user(message));
    }

    let url = sxcu::expand(&sxcu.url, &vars)?;
    if url.trim().is_empty() {
        return Err(user(format!(
            "the uploader returned no URL; response was: {}",
            util::truncate_text(&text, 300)
        ))
        .hint("check the .sxcu URL template (`$json:...$` or `$response$`)"));
    }
    Ok(url.trim().to_string())
}

fn upload_presigned(ctx: &Ctx, url: &str, method: &str, payload: &Payload) -> Result<String> {
    let target = url.replace("{filename}", &util::form_encode(&payload.filename));
    let headers = vec![
        ("Content-Type".to_string(), payload.mime.clone()),
        ("User-Agent".to_string(), format!("ss-belt/{}", crate::VERSION)),
    ];
    let (status, text) = http_send(
        &method.to_ascii_uppercase(),
        &target,
        &headers,
        payload.bytes.clone(),
        Duration::from_secs(ctx.config.upload.timeout_secs),
    )?;
    if !(200..300).contains(&status) {
        return Err(user(format!(
            "presigned upload failed with HTTP {status}: {}",
            util::truncate_text(&text, 300)
        )));
    }
    Ok(target.split('?').next().unwrap_or(&target).to_string())
}

fn build_multipart(
    boundary: &str,
    field: &str,
    payload: &Payload,
    arguments: &[(String, String)],
) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    for (key, value) in arguments {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{key}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field}\"; filename=\"{}\"\r\n",
            payload.filename.replace('"', "")
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", payload.mime).as_bytes());
    body.extend_from_slice(&payload.bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

fn http_send(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
    timeout: Duration,
) -> Result<(u16, String)> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(timeout)
        .build();
    let mut request = agent.request(method, url);
    for (key, value) in headers {
        request = request.set(key, value);
    }
    match request.send_bytes(&body) {
        Ok(response) => {
            let status = response.status();
            let text = response.into_string().unwrap_or_default();
            Ok((status, text))
        }
        Err(ureq::Error::Status(status, response)) => {
            let text = response.into_string().unwrap_or_default();
            Ok((status, text))
        }
        Err(err) => Err(anyhow::Error::new(err).context(format!("uploading to {url}"))),
    }
}

/// Re-encode images through the decoder/encoder pair, which drops EXIF and
/// other ancillary metadata that could leak location or device details.
fn strip_metadata(payload: Payload) -> Payload {
    if payload.mime != "image/png" && payload.mime != "image/jpeg" {
        return payload;
    }
    let Ok(image) = image::load_from_memory(&payload.bytes) else {
        return payload;
    };
    let mut out = Cursor::new(Vec::new());
    let format = if payload.mime == "image/png" {
        image::ImageFormat::Png
    } else {
        image::ImageFormat::Jpeg
    };
    if image.write_to(&mut out, format).is_err() {
        return payload;
    }
    Payload { bytes: out.into_inner(), ..payload }
}

pub fn destination_exists(ctx: &Ctx, name: &str) -> Option<PathBuf> {
    match ctx.config.upload.destinations.get(name) {
        Some(Destination::Sxcu { file }) => Some(util::expand_tilde(file)),
        _ => None,
    }
}

pub fn is_remote(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn one_shot_server(response: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = vec![0u8; 65536];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let (head, body) = response.split_once("\r\n\r\n").unwrap_or((response, ""));
            let wire = format!(
                "{}\r\nContent-Length: {}\r\n\r\n{}",
                head.replace("Content-Length: 28", &format!("Content-Length: {}", body.len())),
                body.len(),
                body
            );
            let _ = stream.write_all(wire.as_bytes());
            let _ = stream.flush();
            request
        });
        (address, handle)
    }

    #[test]
    fn multipart_body_contains_file_and_fields() {
        let payload = Payload {
            filename: "shot.png".into(),
            mime: "image/png".into(),
            bytes: b"PNGDATA".to_vec(),
            is_text: false,
        };
        let body = build_multipart("B", "file", &payload, &[("album".into(), "shots".into())]);
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"album\""));
        assert!(text.contains("name=\"file\"; filename=\"shot.png\""));
        assert!(text.contains("PNGDATA"));
        assert!(text.ends_with("--B--\r\n"));
    }

    #[test]
    fn sxcu_upload_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("region", dir.path());

        let (address, handle) = one_shot_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 28\r\n\r\n{\"data\":{\"url\":\"https://x/y\"}}",
        );

        let sxcu_path = dir.path().join("up.sxcu");
        std::fs::write(
            &sxcu_path,
            format!(
                r#"{{"RequestMethod":"POST","RequestURL":"{address}/u","Body":"MultipartFormData","FileFormName":"file","URL":"$json:data.url$"}}"#
            ),
        )
        .unwrap();

        ctx.config.upload.default_destination = "test".into();
        ctx.config.upload.destinations.insert(
            "test".into(),
            Destination::Sxcu { file: sxcu_path.display().to_string() },
        );

        let image = ctx.stage("capture.png");
        image::GrayImage::from_pixel(4, 4, image::Luma([0])).save(&image).unwrap();
        ctx.value = Value::Image(image);

        run(&mut ctx).unwrap();
        assert_eq!(ctx.value, Value::Url("https://x/y".into()));

        let request = handle.join().unwrap();
        assert!(request.starts_with("POST /u"));
        assert!(request.contains("multipart/form-data"));
    }

    #[test]
    fn http_error_surfaces_the_status() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("region", dir.path());
        let (address, _handle) = one_shot_server(
            "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 7\r\n\r\ntoo big",
        );

        let sxcu_path = dir.path().join("up.sxcu");
        std::fs::write(
            &sxcu_path,
            format!(
                r#"{{"RequestURL":"{address}/u","Body":"Binary","URL":"$response$","ErrorMessage":"server said: $response$"}}"#
            ),
        )
        .unwrap();
        ctx.config.upload.default_destination = "t".into();
        ctx.config.upload.destinations.insert(
            "t".into(),
            Destination::Sxcu { file: sxcu_path.display().to_string() },
        );
        ctx.value = Value::Text("hello".into());

        let err = run(&mut ctx).unwrap_err().to_string();
        assert!(err.contains("413"), "{err}");
        assert!(err.contains("too big"), "{err}");
    }

    #[test]
    fn presigned_put_reports_the_object_url() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("region", dir.path());
        let (address, handle) = one_shot_server("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        ctx.config.upload.default_destination = "r2".into();
        ctx.config.upload.destinations.insert(
            "r2".into(),
            Destination::Presigned {
                url: format!("{address}/bucket/{{filename}}?X-Amz-Signature=abc"),
                method: "PUT".into(),
            },
        );
        ctx.value = Value::Text("payload".into());

        run(&mut ctx).unwrap();
        assert_eq!(ctx.value, Value::Url(format!("{address}/bucket/region.txt")));

        let request = handle.join().unwrap();
        assert!(request.starts_with("PUT /bucket/region.txt?X-Amz-Signature=abc"));
    }

    #[test]
    fn metadata_stripping_round_trips_an_image() {
        let payload = Payload {
            filename: "a.png".into(),
            mime: "image/png".into(),
            bytes: {
                let mut cursor = Cursor::new(Vec::new());
                image::GrayImage::from_pixel(2, 2, image::Luma([7]))
                    .write_to(&mut cursor, image::ImageFormat::Png)
                    .unwrap();
                cursor.into_inner()
            },
            is_text: false,
        };
        let stripped = strip_metadata(payload);
        assert!(image::load_from_memory(&stripped.bytes).is_ok());
    }
}
