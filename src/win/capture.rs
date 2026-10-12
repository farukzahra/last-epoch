use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    SRCCOPY,
};

/// A pixel counts as black when all three channels are below this.
const BLACK_LEVEL: u8 = 16;

/// True when at least `threshold_percent` of the pixels are black.
///
/// An empty buffer is **not** black: an empty capture is a failed capture, and
/// the UI must not report "exclusive fullscreen" for it.
pub fn is_mostly_black(pixels: &[u8], threshold_percent: u8) -> bool {
    let total = pixels.len() / 4;
    if total == 0 {
        return false;
    }
    let dark = pixels
        .chunks_exact(4)
        .filter(|px| px[0] < BLACK_LEVEL && px[1] < BLACK_LEVEL && px[2] < BLACK_LEVEL)
        .count();
    (dark * 100) / total >= threshold_percent as usize
}

/// Grab a screen rectangle as top-down BGRA rows. `None` when the OS refuses.
pub fn capture_region(rect: (i32, i32, i32, i32)) -> Option<Vec<u8>> {
    let (left, top, right, bottom) = rect;
    let width = right - left;
    let height = bottom - top;
    if width <= 0 || height <= 0 {
        return None;
    }

    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);
        if mem.is_invalid() {
            ReleaseDC(None, screen);
            return None;
        }
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            return None;
        }
        let old = SelectObject(mem, HGDIOBJ(bitmap.0));

        // Blit the screen rectangle into the compatible bitmap we selected.
        let _ = BitBlt(mem, 0, 0, width, height, screen, left, top, SRCCOPY);

        let mut info = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height; // negative keeps rows top-down
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB.0;

        let mut buffer = vec![0u8; (width * height * 4) as usize];
        let scanned = GetDIBits(
            mem,
            bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);

        if scanned == 0 {
            None
        } else {
            Some(buffer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_all_black_buffer_is_mostly_black() {
        let pixels = vec![0u8; 4 * 100];
        assert!(is_mostly_black(&pixels, 95));
    }

    #[test]
    fn a_buffer_with_light_pixels_is_not_mostly_black() {
        let mut pixels = vec![0u8; 4 * 100];
        for i in 0..50 {
            pixels[i * 4] = 200;
            pixels[i * 4 + 1] = 180;
            pixels[i * 4 + 2] = 90;
        }
        assert!(!is_mostly_black(&pixels, 95));
    }

    #[test]
    fn an_empty_buffer_is_not_treated_as_black() {
        assert!(!is_mostly_black(&[], 95));
    }
}
