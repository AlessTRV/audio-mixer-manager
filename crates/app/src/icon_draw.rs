// Icon drawing, shared by the app (`icon.rs`) and `build.rs` through `include!`.
// Only `std` may be used here, and no inner attributes or doc comments.

/// Speaker glyph with two sound waves, white on transparent, `size`×`size` RGBA8.
#[must_use]
pub fn speaker_rgba(size: u32) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    #[allow(clippy::cast_precision_loss)]
    let scale = (size * SAMPLES) as f32;
    for y in 0..size {
        for x in 0..size {
            let mut hits = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    #[allow(clippy::cast_precision_loss)]
                    let u = ((x * SAMPLES + sx) as f32 + 0.5) / scale;
                    #[allow(clippy::cast_precision_loss)]
                    let v = ((y * SAMPLES + sy) as f32 + 0.5) / scale;
                    if inside(u, v) {
                        hits += 1;
                    }
                }
            }
            let alpha = hits * 255 / (SAMPLES * SAMPLES);
            let alpha = u8::try_from(alpha).unwrap_or(u8::MAX);
            rgba.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    rgba
}

/// Whether the point `(u, v)` in unit coordinates belongs to the glyph.
fn inside(u: f32, v: f32) -> bool {
    let dy = (v - 0.5).abs();
    // Speaker body.
    let body = (0.10..=0.30).contains(&u) && dy <= 0.13;
    // Cone, widening from the body to the front.
    let cone = (0.30..=0.52).contains(&u) && dy <= 0.13 + (u - 0.30) / 0.22 * 0.24;
    // Two arcs centred on the front of the cone, within ±50°.
    let (cx, cy) = (0.52, 0.5);
    let r = ((u - cx).powi(2) + (v - cy).powi(2)).sqrt();
    let angle = (v - cy).atan2(u - cx).abs();
    let in_sector = u > cx && angle < 50f32.to_radians();
    let waves = in_sector && ((0.14..=0.21).contains(&r) || (0.29..=0.36).contains(&r));
    body || cone || waves
}

/// App icon: white speaker on a rounded accent square, `size`×`size` RGBA8.
/// Used for the window, the executable and the installer.
#[must_use]
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    const ACCENT: [u8; 3] = [0x7C, 0x6C, 0xFF];
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    #[allow(clippy::cast_precision_loss)]
    let scale = (size * SAMPLES) as f32;
    for y in 0..size {
        for x in 0..size {
            let (mut square, mut glyph) = (0u32, 0u32);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    #[allow(clippy::cast_precision_loss)]
                    let u = ((x * SAMPLES + sx) as f32 + 0.5) / scale;
                    #[allow(clippy::cast_precision_loss)]
                    let v = ((y * SAMPLES + sy) as f32 + 0.5) / scale;
                    if in_rounded_square(u, v) {
                        square += 1;
                        if inside((u - 0.16) / 0.68, (v - 0.16) / 0.68) {
                            glyph += 1;
                        }
                    }
                }
            }
            let total = SAMPLES * SAMPLES;
            // Blend white over the accent by glyph coverage within the square.
            let mix = |accent: u8| -> u8 {
                let value = (u32::from(accent) * (square - glyph) + 255 * glyph) / square.max(1);
                u8::try_from(value).unwrap_or(u8::MAX)
            };
            let alpha = u8::try_from(square * 255 / total).unwrap_or(u8::MAX);
            rgba.extend_from_slice(&[mix(ACCENT[0]), mix(ACCENT[1]), mix(ACCENT[2]), alpha]);
        }
    }
    rgba
}

/// Rounded square filling the unit cell with a small margin.
fn in_rounded_square(u: f32, v: f32) -> bool {
    const MARGIN: f32 = 0.03;
    const RADIUS: f32 = 0.22;
    let half = 0.5 - MARGIN - RADIUS;
    let dx = ((u - 0.5).abs() - half).max(0.0);
    let dy = ((v - 0.5).abs() - half).max(0.0);
    dx * dx + dy * dy <= RADIUS * RADIUS
}
