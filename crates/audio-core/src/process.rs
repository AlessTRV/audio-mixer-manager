//! Process information: pid to executable path, display name and icon.

use std::ffi::{OsString, c_void};
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, HBITMAP, HDC, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::{
    FILE_FLAGS_AND_ATTRIBUTES, GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::{HSTRING, PCWSTR, PWSTR, w};

/// Name used for the system sounds session (pid 0).
const SYSTEM_SOUNDS_EXE: &str = "system sounds";
const SYSTEM_SOUNDS_DISPLAY: &str = "System Sounds";

/// Maximum length of an extended-length path, in UTF-16 units.
const MAX_PATH_LEN: u32 = 32_768;

/// What we know about a process that owns an audio session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub exe_path: Option<PathBuf>,
    /// Lowercase file name, e.g. `spotify.exe`; `system sounds` for pid 0;
    /// `pid <n>` when the executable cannot be read.
    pub exe_name: String,
    /// `FileDescription` from the version resource, else the file stem;
    /// `System Sounds` for pid 0.
    pub display_name: String,
}

/// Collects information about `pid`. Never fails: missing pieces fall back
/// to placeholder names.
#[must_use]
pub fn process_info(pid: u32) -> ProcessInfo {
    if pid == 0 {
        return ProcessInfo {
            pid,
            exe_path: None,
            exe_name: SYSTEM_SOUNDS_EXE.to_owned(),
            display_name: SYSTEM_SOUNDS_DISPLAY.to_owned(),
        };
    }

    let fallback = format!("pid {pid}");
    let Some(path) = exe_path(pid) else {
        return ProcessInfo {
            pid,
            exe_path: None,
            exe_name: fallback.clone(),
            display_name: fallback,
        };
    };

    let exe_name = path.file_name().map_or_else(
        || fallback.clone(),
        |name| name.to_string_lossy().to_lowercase(),
    );
    let display_name = file_description(&path)
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .filter(|stem| !stem.is_empty())
        })
        .unwrap_or(fallback);

    ProcessInfo {
        pid,
        exe_path: Some(path),
        exe_name,
        display_name,
    }
}

/// An icon as straight (non-premultiplied) RGBA8 pixels, rows top-down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconRgba {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Large (32x32) shell icon of an executable.
///
/// Calls `SHGetFileInfoW`, which expects COM to be initialized on the thread.
#[must_use]
pub fn exe_icon(path: &Path) -> Option<IconRgba> {
    let wide = HSTRING::from(path);
    let mut info = SHFILEINFOW::default();
    let info_size = u32::try_from(size_of::<SHFILEINFOW>()).ok()?;
    // SAFETY: `wide` is a NUL-terminated path that outlives the call and
    // `info` is a properly sized, writable SHFILEINFOW.
    let found = unsafe {
        SHGetFileInfoW(
            &wide,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&raw mut info),
            info_size,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if found == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let icon = IconGuard(info.hIcon);
    icon_to_rgba(icon.0)
}

/// Full Win32 path of the executable of `pid`.
fn exe_path(pid: u32) -> Option<PathBuf> {
    // SAFETY: plain Win32 call; the returned handle is owned by the guard.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let handle = HandleGuard(handle);

    let mut buffer = vec![0u16; MAX_PATH_LEN as usize];
    let mut len = MAX_PATH_LEN;
    // SAFETY: `buffer` holds `len` writable UTF-16 units and `handle` is open.
    unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &raw mut len,
        )
    }
    .ok()?;
    buffer.truncate(len as usize);
    if buffer.is_empty() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&buffer)))
}

/// `FileDescription` from the version resource of `path`, if present and not empty.
fn file_description(path: &Path) -> Option<String> {
    let wide = HSTRING::from(path);
    // SAFETY: `wide` is a NUL-terminated path; the handle argument is optional.
    let size = unsafe { GetFileVersionInfoSizeW(&wide, None) };
    if size == 0 {
        return None;
    }
    // u32 storage keeps the block DWORD-aligned, as the version structures expect.
    let mut block = vec![0u32; (size as usize).div_ceil(4)];
    // SAFETY: `block` has at least `size` writable bytes.
    unsafe { GetFileVersionInfoW(&wide, None, size, block.as_mut_ptr().cast()) }.ok()?;
    let mut candidates: Vec<(u16, u16)> = Vec::new();
    if let Some(translations) = query_value(&block, w!("\\VarFileInfo\\Translation"), true) {
        candidates.extend(
            translations
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[lang, cp]| (lang, cp)),
        );
    }
    // Common fallbacks: US English with Unicode / Windows-1252, neutral Unicode.
    candidates.extend([(0x0409, 0x04B0), (0x0409, 0x04E4), (0x0000, 0x04B0)]);

    for (lang, codepage) in candidates {
        let sub_block = HSTRING::from(format!(
            "\\StringFileInfo\\{lang:04x}{codepage:04x}\\FileDescription"
        ));
        if let Some(units) = query_value(&block, PCWSTR(sub_block.as_ptr()), false) {
            // Some binaries report a length past the terminator (Spotify reports it in
            // bytes), which would pull in the next entry: stop at the first NUL.
            let units = units.split(|&unit| unit == 0).next().unwrap_or_default();
            let text = String::from_utf16_lossy(units);
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_owned());
            }
        }
    }
    None
}

/// Runs `VerQueryValueW` on a version block and returns the value as UTF-16 units.
///
/// `len_in_bytes` is true for binary values such as `\VarFileInfo\Translation`
/// (length reported in bytes) and false for string values (length in characters).
fn query_value(block: &[u32], sub_block: PCWSTR, len_in_bytes: bool) -> Option<&[u16]> {
    let mut value: *mut c_void = std::ptr::null_mut();
    let mut len = 0u32;
    // SAFETY: `block` was filled by GetFileVersionInfoW and `sub_block` is a
    // NUL-terminated string valid for the call.
    let ok = unsafe {
        VerQueryValueW(
            block.as_ptr().cast(),
            sub_block,
            &raw mut value,
            &raw mut len,
        )
    };
    if !ok.as_bool() || value.is_null() || len == 0 {
        return None;
    }
    let units = if len_in_bytes {
        len as usize / 2
    } else {
        len as usize
    };
    let block_start: *const u8 = block.as_ptr().cast();
    let block_end = block_start.wrapping_add(size_of_val(block));
    let value_start: *const u8 = value.cast_const().cast();
    let value_end = value_start.wrapping_add(units * 2);
    if value_start < block_start || value_end > block_end {
        return None;
    }
    // SAFETY: the range was checked to lie inside `block`, which outlives the
    // returned slice; the block is DWORD-aligned and version values are
    // 2-byte aligned within it.
    Some(unsafe { std::slice::from_raw_parts(value.cast_const().cast::<u16>(), units) })
}

/// Converts a color icon into RGBA8 pixels.
fn icon_to_rgba(icon: HICON) -> Option<IconRgba> {
    let mut icon_info = ICONINFO::default();
    // SAFETY: `icon` is a valid icon handle and `icon_info` is writable.
    unsafe { GetIconInfo(icon, &raw mut icon_info) }.ok()?;
    // GetIconInfo created both bitmaps: they are ours to delete.
    let color = BitmapGuard(icon_info.hbmColor);
    let mask = BitmapGuard(icon_info.hbmMask);
    if color.0.is_invalid() {
        // Monochrome icon: not supported, the caller shows a placeholder.
        return None;
    }

    let mut bitmap = BITMAP::default();
    let bitmap_size = i32::try_from(size_of::<BITMAP>()).ok()?;
    // SAFETY: `color.0` is a valid bitmap and `bitmap` has the declared size.
    let written =
        unsafe { GetObjectW(color.0.into(), bitmap_size, Some((&raw mut bitmap).cast())) };
    if written == 0 {
        return None;
    }
    let width = u32::try_from(bitmap.bmWidth).ok().filter(|w| *w > 0)?;
    let height = u32::try_from(bitmap.bmHeight).ok().filter(|h| *h > 0)?;

    // SAFETY: requesting the screen DC; released by the guard.
    let dc = DcGuard(unsafe { GetDC(None) });
    if dc.0.is_invalid() {
        return None;
    }

    let bgra = read_bits(dc.0, color.0, width, height)?;
    let mut rgba = Vec::with_capacity(bgra.len());
    for &[b, g, r, a] in bgra.as_chunks::<4>().0 {
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    if rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 0) {
        // No alpha channel: derive it from the AND mask (set bit = transparent).
        match read_bits(dc.0, mask.0, width, height) {
            Some(mask_bits) => {
                let pixels = rgba.as_chunks_mut::<4>().0.iter_mut();
                for (px, m) in pixels.zip(mask_bits.as_chunks::<4>().0) {
                    px[3] = if m[0] == 0 { 255 } else { 0 };
                }
            }
            None => {
                for px in rgba.as_chunks_mut::<4>().0 {
                    px[3] = 255;
                }
            }
        }
    }

    Some(IconRgba {
        width,
        height,
        rgba,
    })
}

/// Reads `bitmap` as 32 bpp top-down BGRA.
fn read_bits(dc: HDC, bitmap: HBITMAP, width: u32, height: u32) -> Option<Vec<u8>> {
    if bitmap.is_invalid() {
        return None;
    }
    let signed_width = i32::try_from(width).ok()?;
    let signed_height = i32::try_from(height).ok()?;
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: signed_width,
            // Negative height = top-down rows.
            biHeight: -signed_height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut buffer = vec![0u8; width as usize * height as usize * 4];
    // SAFETY: `buffer` holds width * height 32-bit pixels, `info` describes
    // exactly that layout and `bitmap` is not selected into any DC.
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height,
            Some(buffer.as_mut_ptr().cast()),
            &raw mut info,
            DIB_RGB_COLORS,
        )
    };
    (lines == signed_height).then_some(buffer)
}

/// Closes a process handle on drop.
struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: the handle was returned by OpenProcess and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Destroys an icon on drop.
struct IconGuard(HICON);

impl Drop for IconGuard {
    fn drop(&mut self) {
        // SAFETY: the icon was created by SHGetFileInfoW and is destroyed once.
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

/// Deletes a GDI bitmap on drop (no-op for null handles).
struct BitmapGuard(HBITMAP);

impl Drop for BitmapGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: the bitmap was created by GetIconInfo and is deleted once.
            let _ = unsafe { DeleteObject(self.0.into()) };
        }
    }
}

/// Releases a screen DC on drop.
struct DcGuard(HDC);

impl Drop for DcGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: the DC was obtained with GetDC(None) and is released once.
            let _ = unsafe { ReleaseDC(None, self.0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_sounds() {
        let info = process_info(0);
        assert_eq!(info.pid, 0);
        assert_eq!(info.exe_path, None);
        assert_eq!(info.exe_name, "system sounds");
        assert_eq!(info.display_name, "System Sounds");
    }

    #[test]
    fn current_process() {
        let pid = std::process::id();
        let info = process_info(pid);
        assert_eq!(info.pid, pid);
        let path = info
            .exe_path
            .expect("own executable path should be readable");
        assert!(path.exists(), "{} does not exist", path.display());
        assert_eq!(info.exe_name, info.exe_name.to_lowercase());
        assert!(
            std::path::Path::new(&info.exe_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe")),
            "{}",
            info.exe_name
        );
        assert_ne!(info.display_name, "");
    }

    #[test]
    fn missing_process_falls_back() {
        // Windows pids are multiples of 4 and never get this high.
        let pid = 0xFFFF_FFF0;
        let info = process_info(pid);
        assert_eq!(info.exe_path, None);
        assert_eq!(info.exe_name, format!("pid {pid}"));
        assert_eq!(info.display_name, format!("pid {pid}"));
    }
}
