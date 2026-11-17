//! Embeds the icon, version information and manifest into the executable.
//!
//! The icon is drawn by the same code the app uses at run time
//! (`src/icon_draw.rs`) and written as a multi-size `.ico` into `OUT_DIR`.

use std::path::PathBuf;
use std::{env, fs, io};

#[allow(dead_code)]
mod icon {
    include!("src/icon_draw.rs");
}

const ICO_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/icon_draw.rs");
    println!("cargo:rerun-if-changed=app.manifest");

    let out_dir =
        PathBuf::from(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("OUT_DIR unset"))?);
    let ico_path = out_dir.join("app.ico");
    fs::write(&ico_path, ico(&ICO_SIZES))?;

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(&ico_path.to_string_lossy())
        .set_manifest_file("app.manifest")
        .set("ProductName", "Audio Mixer Manager")
        .set("FileDescription", "Audio Mixer Manager")
        .set("InternalName", "audiomixermanager")
        .set("OriginalFilename", "audiomixermanager.exe");
    res.compile()
}

/// A Windows `.ico` with one 32-bit BMP image per size.
fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&size| bmp_image(size)).collect();
    let count = u16::try_from(sizes.len()).unwrap_or(u16::MAX);

    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    out.extend_from_slice(&count.to_le_bytes());

    let mut offset = 6 + 16 * sizes.len();
    for (&size, image) in sizes.iter().zip(&images) {
        // 256 is stored as 0.
        let dim = u8::try_from(size).unwrap_or(0);
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&u32::try_from(image.len()).unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset).unwrap_or(0).to_le_bytes());
        offset += image.len();
    }
    for image in images {
        out.extend_from_slice(&image);
    }
    out
}

/// BITMAPINFOHEADER + bottom-up BGRA pixels + an all-zero AND mask.
fn bmp_image(size: u32) -> Vec<u8> {
    let rgba = icon::app_icon_rgba(size);
    let width = size as usize;
    let mask_row = width.div_ceil(32) * 4;
    let pixels_len = width * width * 4;
    let mask_len = mask_row * width;

    let mut out = Vec::with_capacity(40 + pixels_len + mask_len);
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&i32::try_from(size).unwrap_or(0).to_le_bytes());
    // Height covers the colour image and the mask.
    out.extend_from_slice(&i32::try_from(size * 2).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(
        &u32::try_from(pixels_len + mask_len)
            .unwrap_or(0)
            .to_le_bytes(),
    );
    out.extend_from_slice(&[0; 16]); // resolution and palette fields

    for row in (0..width).rev() {
        for &[r, g, b, a] in rgba[row * width * 4..(row + 1) * width * 4]
            .as_chunks::<4>()
            .0
        {
            out.extend_from_slice(&[b, g, r, a]);
        }
    }
    out.resize(out.len() + mask_len, 0);
    out
}
