use anyhow::{Context, Result};

use crate::error::user;
use crate::steps::{Ctx, Probe, Support, Value};
use crate::error::HintExt;

pub fn support(_probe: &Probe<'_>) -> Support {
    Support::Ready
}

pub fn scan_run(ctx: &mut Ctx) -> Result<()> {
    let path = ctx.require_image("qr.scan")?;

    if ctx.dry_run {
        ctx.note(format!("dry-run: would decode QR codes in {}", path.display()));
        ctx.value = Value::Text("<qr>".into());
        return Ok(());
    }

    let image = image::open(&path)
        .with_context(|| format!("reading image {}", path.display()))?
        .to_luma8();

    let mut prepared = rqrr::PreparedImage::prepare(image);
    let grids = prepared.detect_grids();

    let mut texts = Vec::new();
    for grid in grids {
        match grid.decode() {
            Ok((_meta, content)) => texts.push(content),
            Err(err) => ctx.note(format!("a QR grid could not be decoded: {err}")),
        }
    }

    if texts.is_empty() {
        return Err(user("no QR code found in the capture").hint(
            "make sure the code is fully inside the selection and not blurred",
        ));
    }

    let text = texts.join("\n");
    ctx.note(format!("decoded {} QR code(s)", texts.len()));
    ctx.value = Value::Text(text);
    Ok(())
}

pub fn make_run(ctx: &mut Ctx) -> Result<()> {
    let text = ctx.require_text("qr.make")?;
    if text.trim().is_empty() {
        return Err(user("cannot encode an empty string"));
    }

    if ctx.dry_run {
        ctx.note("dry-run: would render a QR code".to_string());
        ctx.value = Value::Image(ctx.stage("qr.png"));
        return Ok(());
    }

    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M)
        .with_context(|| "the text is too long for a QR code")?;

    let modules = code.width() as u32;
    let modules_colors = code.to_colors();
    let scale: u32 = 8;
    let border: u32 = 4 * scale;
    let size = modules * scale + border * 2;

    let mut canvas = image::GrayImage::from_pixel(size, size, image::Luma([255]));
    for (index, color) in modules_colors.iter().enumerate() {
        if *color != qrcode::Color::Dark {
            continue;
        }
        let x = (index as u32) % modules;
        let y = (index as u32) / modules;
        for dy in 0..scale {
            for dx in 0..scale {
                canvas.put_pixel(border + x * scale + dx, border + y * scale + dy, image::Luma([0]));
            }
        }
    }

    let path = ctx.stage("qr.png");
    canvas.save(&path).with_context(|| "writing the QR image")?;
    ctx.note(format!("rendered a {size}x{size} QR code"));
    ctx.value = Value::Image(path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::testing;

    #[test]
    fn a_rendered_code_decodes_back_to_the_same_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = testing::ctx_for("qr_make", dir.path());
        ctx.value = Value::Text("https://example.com/ss-belt?x=1".into());
        make_run(&mut ctx).unwrap();

        let image_path = match &ctx.value {
            Value::Image(path) => path.clone(),
            other => panic!("unexpected {other:?}"),
        };

        let mut scan = testing::ctx_for("qr_scan", dir.path());
        scan.value = Value::Image(image_path);
        scan_run(&mut scan).unwrap();
        assert_eq!(scan.value, Value::Text("https://example.com/ss-belt?x=1".into()));
    }

    #[test]
    fn an_image_without_codes_reports_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let blank = dir.path().join("blank.png");
        image::GrayImage::from_pixel(64, 64, image::Luma([255])).save(&blank).unwrap();

        let mut ctx = testing::ctx_for("qr_scan", dir.path());
        ctx.value = Value::Image(blank);
        let err = scan_run(&mut ctx).unwrap_err();
        assert!(err.to_string().contains("no QR code"));
    }
}
