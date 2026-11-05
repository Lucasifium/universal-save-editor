//! Procedurally drawn app icon: a floppy disk with a pencil on a rounded gradient tile.
//! Shared by the window icon (runtime) and the .exe icon (build.rs).

type Rgba = [f32; 4];

fn rounded_rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    let (dx, dy) = (x - cx, y - cy);
    x >= x0 && x <= x1 && y >= y0 && y <= y1 && dx * dx + dy * dy <= r * r
}

fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> (f32, f32) {
    let (vx, vy) = (bx - ax, by - ay);
    let t = (((px - ax) * vx + (py - ay) * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
    let (cx, cy) = (ax + vx * t, ay + vy * t);
    (((px - cx).powi(2) + (py - cy).powi(2)).sqrt(), t)
}

fn over(dst: Rgba, src: Rgba) -> Rgba {
    let a = src[3] + dst[3] * (1.0 - src[3]);
    if a <= 0.0 {
        return [0.0; 4];
    }
    let mix = |i: usize| (src[i] * src[3] + dst[i] * dst[3] * (1.0 - src[3])) / a;
    [mix(0), mix(1), mix(2), a]
}

fn hex(c: u32) -> Rgba {
    [((c >> 16) & 0xFF) as f32 / 255.0, ((c >> 8) & 0xFF) as f32 / 255.0, (c & 0xFF) as f32 / 255.0, 1.0]
}

/// Colour of the icon at normalised coordinates (0..1, 0..1).
fn sample(x: f32, y: f32) -> Rgba {
    let mut c: Rgba = [0.0; 4];

    // Background tile with a top-left → bottom-right gradient.
    if rounded_rect(x, y, 0.03, 0.03, 0.97, 0.97, 0.2) {
        let t = ((x + y) / 2.0).clamp(0.0, 1.0);
        let (a, b) = (hex(0x3D8BFD), hex(0x7B4FE0));
        c = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1.0];
    }

    // Floppy body (white, top-right corner clipped).
    let (fx0, fy0, fx1, fy1) = (0.2, 0.18, 0.74, 0.76);
    let clipped = x + (fy0 - y) > fx1 - 0.1;
    if rounded_rect(x, y, fx0, fy0, fx1, fy1, 0.05) && !clipped {
        c = over(c, hex(0xF5F7FF));
        // Metal shutter.
        if rounded_rect(x, y, 0.32, fy0, 0.58, 0.36, 0.015) {
            c = over(c, hex(0x5B6B8C));
            if rounded_rect(x, y, 0.48, 0.21, 0.54, 0.33, 0.01) {
                c = over(c, hex(0xF5F7FF));
            }
        }
        // Label area with two text lines.
        if rounded_rect(x, y, 0.28, 0.47, 0.66, 0.76, 0.02) {
            c = over(c, hex(0xDCE4F7));
            for ly in [0.55, 0.64] {
                if (y - ly).abs() < 0.018 && x > 0.33 && x < 0.61 {
                    c = over(c, hex(0x9AA8C7));
                }
            }
        }
    }

    // Pencil, diagonal across the bottom-right.
    let (ax, ay, bx, by) = (0.56, 0.86, 0.86, 0.56);
    let (d, t) = seg_dist(x, y, ax, ay, bx, by);
    if t > 0.0 && t < 1.0 {
        if t < 0.16 {
            // Sharpened tip: tapers to a point at (ax, ay).
            let w = 0.075 * (t / 0.16);
            if d < w {
                c = over(c, if t < 0.07 { hex(0x2B2B3A) } else { hex(0xF2D7A6) });
            }
        } else if d < 0.075 {
            let col = if t > 0.86 { hex(0xF06A8A) } else if t > 0.8 { hex(0xC9CED8) } else { hex(0xFFB547) };
            c = over(c, col);
        }
    }
    c
}

/// Render the icon at `size`×`size` as straight (non-premultiplied) RGBA8, 4×4 supersampled.
pub fn rgba(size: u32) -> Vec<u8> {
    const SS: u32 = 4;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let mut acc = [0.0f32; 4];
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = (px as f32 + (sx as f32 + 0.5) / SS as f32) / size as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / SS as f32) / size as f32;
                    let s = sample(x, y);
                    for i in 0..3 {
                        acc[i] += s[i] * s[3];
                    }
                    acc[3] += s[3];
                }
            }
            let n = (SS * SS) as f32;
            let a = acc[3] / n;
            for i in 0..3 {
                let v = if acc[3] > 0.0 { acc[i] / acc[3] } else { 0.0 };
                out.push((v * 255.0).round() as u8);
            }
            out.push((a * 255.0).round() as u8);
        }
    }
    out
}

/// Build a Windows .ico (32-bit BMP entries) containing several sizes.
#[allow(dead_code)]
pub fn ico(sizes: &[u32]) -> Vec<u8> {
    let mut images: Vec<Vec<u8>> = Vec::new();
    for &s in sizes {
        let px = rgba(s);
        let mask_row = (s.div_ceil(32) * 4) as usize;
        let mut img = Vec::new();
        for v in [40u32, s, s * 2] {
            img.extend_from_slice(&v.to_le_bytes());
        }
        img.extend_from_slice(&1u16.to_le_bytes());
        img.extend_from_slice(&32u16.to_le_bytes());
        img.extend_from_slice(&[0u8; 24]);
        for row in (0..s).rev() {
            for col in 0..s {
                let i = ((row * s + col) * 4) as usize;
                img.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
            }
        }
        img.extend(std::iter::repeat_n(0u8, mask_row * s as usize));
        images.push(img);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&images) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(img.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        out.extend_from_slice(&img);
    }
    out
}

/// The icon as a PNG (uncompressed deflate; it's only used at build time for Android and macOS).
#[allow(dead_code)]
pub fn png(size: u32) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    let px = rgba(size);
    let mut raw = Vec::with_capacity(px.len() + size as usize);
    for row in px.chunks(size as usize * 4) {
        raw.push(0); // no filter
        raw.extend_from_slice(row);
    }
    // zlib stream of stored blocks.
    let mut z = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, b) in blocks.iter().enumerate() {
        z.push((i + 1 == blocks.len()) as u8);
        z.extend_from_slice(&(b.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(b.len() as u16)).to_le_bytes());
        z.extend_from_slice(b);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8-bit RGBA
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}
