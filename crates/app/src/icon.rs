//! Application and tray icons, drawn in code.

include!("icon_draw.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_expected_size_and_some_ink() {
        let rgba = speaker_rgba(32);
        assert_eq!(rgba.len(), 32 * 32 * 4);
        assert!(rgba.as_chunks::<4>().0.iter().any(|px| px[3] == 255));
        assert!(rgba.as_chunks::<4>().0.iter().any(|px| px[3] == 0));
    }

    #[test]
    fn app_icon_is_opaque_in_the_middle_and_clear_in_the_corner() {
        let rgba = app_icon_rgba(32);
        let alpha = |x: usize, y: usize| rgba[(y * 32 + x) * 4 + 3];
        assert_eq!(alpha(16, 16), 255);
        assert_eq!(alpha(0, 0), 0);
    }
}
