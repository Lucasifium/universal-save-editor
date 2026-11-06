//! Procedurally drawn app icon: a gold key between two sliders on a dark rounded tile.
//! Shared by the window icon (runtime) and the .exe icon (build.rs).

type Rgba = [f32; 4];

fn rounded_rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    let (dx, dy) = (x - cx, y - cy);
    x >= x0 && x <= x1 && y >= y0 && y <= y1 && dx * dx + dy * dy <= r * r
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

fn circle(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> bool {
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

/// A vertical bar with round ends from `y0` to `y1`, `w` wide, centred on `cx`.
fn vbar(x: f32, y: f32, cx: f32, y0: f32, y1: f32, w: f32) -> bool {
    rounded_rect(x, y, cx - w / 2.0, y0, cx + w / 2.0, y1, w / 2.0)
}

fn heart(x: f32, y: f32, cx: f32, cy: f32, s: f32) -> bool {
    let (u, v) = ((x - cx) / s, -(y - cy) / s);
    (u * u + v * v - 1.0).powi(3) - u * u * v.powi(3) <= 0.0
}

fn star(x: f32, y: f32, cx: f32, cy: f32, r: f32) -> bool {
    // Even-odd test against the ten vertices of a five-pointed star.
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.42 };
            (cx + rr * a.sin(), cy - rr * a.cos())
        })
        .collect();
    let mut inside = false;
    for i in 0..10 {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % 10];
        if (y0 > y) != (y1 > y) && x < x0 + (y - y0) / (y1 - y0) * (x1 - x0) {
            inside = !inside;
        }
    }
    inside
}

/// Colour of the icon at normalised coordinates (0..1, 0..1).
fn sample(x: f32, y: f32) -> Rgba {
    let mut c: Rgba = [0.0; 4];
    let ink = hex(0x26204F);

    // Background tile, dark navy to violet.
    if rounded_rect(x, y, 0.03, 0.03, 0.97, 0.97, 0.2) {
        let t = ((x + y) / 2.0).clamp(0.0, 1.0);
        let (a, b) = (hex(0x1A2A48), hex(0x34226B));
        c = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1.0];
    }

    // Slider tracks (left and right); the part below each knob takes the knob's colour.
    let (top, bottom) = (0.17, 0.84);
    for (cx, ky, col) in [(0.255, 0.64, hex(0xFF5470)), (0.745, 0.49, hex(0x4FE08A))] {
        if vbar(x, y, cx, top, bottom, 0.032) {
            c = over(c, [1.0, 1.0, 1.0, 0.16]);
        }
        if vbar(x, y, cx, ky, bottom, 0.032) {
            c = over(c, col);
        }
        if circle(x, y, cx, ky, 0.09) {
            c = over(c, col);
        }
    }
    if heart(x, y, 0.255, 0.65, 0.034) {
        c = over(c, ink);
    }
    if star(x, y, 0.745, 0.49, 0.052) {
        c = over(c, ink);
    }

    // The key: ring, shaft and two teeth, in the middle.
    let gold = hex(0xFFC13D);
    let (kx, ring_y) = (0.5, 0.3);
    if circle(x, y, kx, ring_y, 0.146) || vbar(x, y, kx, ring_y, 0.88, 0.064) {
        c = over(c, gold);
    }
    for (ty0, ty1) in [(0.66, 0.705), (0.76, 0.805)] {
        if rounded_rect(x, y, kx, ty0, kx + 0.115, ty1, 0.012) {
            c = over(c, gold);
        }
    }
    if circle(x, y, kx, ring_y, 0.082) {
        c = over(c, ink);
    }
    if circle(x, y, kx, ring_y, 0.036) {
        c = over(c, gold);
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
