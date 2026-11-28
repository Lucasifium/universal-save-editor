//! Testing aid: the window saving a picture of itself.

use super::*;

/// Testing aid: with SAVE_EDITOR_SHOT=<file.png> set, the window saves a picture of
/// itself a few seconds after starting and then closes (SAVE_EDITOR_SHOT_AFTER = seconds,
/// default 4). Works where screen capture doesn't (e.g. while the display is off).
#[cfg(not(target_os = "android"))]
pub(super) fn self_shot(ctx: &egui::Context) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ASKED: AtomicBool = AtomicBool::new(false);
    let Some(out) = std::env::var_os("SAVE_EDITOR_SHOT") else { return };
    for e in ctx.input(|i| i.raw.events.clone()) {
        if let egui::Event::Screenshot { image, .. } = e {
            let _ = std::fs::write(&out, png(&image));
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    let after: f64 = std::env::var("SAVE_EDITOR_SHOT_AFTER").ok().and_then(|s| s.parse().ok()).unwrap_or(4.0);
    if ctx.input(|i| i.time) > after && !ASKED.swap(true, Ordering::SeqCst) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}

/// A plain RGBA PNG.
#[cfg(not(target_os = "android"))]
pub(super) fn png(img: &egui::ColorImage) -> Vec<u8> {
    use std::io::Write;
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut crc = flate2::Crc::new();
        crc.update(kind);
        crc.update(data);
        out.extend(kind);
        out.extend(data);
        out.extend(crc.sum().to_be_bytes());
    }
    let [w, h] = img.size;
    let mut raw = Vec::with_capacity((w * 4 + 1) * h);
    for row in img.pixels.chunks(w) {
        raw.push(0);
        for p in row {
            raw.extend(p.to_srgba_unmultiplied());
        }
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    let _ = z.write_all(&raw);
    let idat = z.finish().unwrap_or_default();
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend((w as u32).to_be_bytes());
    ihdr.extend((h as u32).to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}
