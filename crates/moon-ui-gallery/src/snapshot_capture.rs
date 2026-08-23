//! Snapshot directory preparation and platform image capture.

use gpui::Window;

/// Creates the snapshot directory and removes only its existing PNG files.
///
/// Returns an error describing the filesystem operation and path that failed.
pub(super) fn clear_snapshot_dir(dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("create {}: {err}", dir.display()))?;
    let entries = std::fs::read_dir(dir).map_err(|err| format!("read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("read dir entry {}: {err}", dir.display()))?;
        if entry.path().extension().and_then(|ext| ext.to_str()) == Some("png") {
            std::fs::remove_file(entry.path())
                .map_err(|err| format!("remove {}: {err}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Captures the current window through GPUI or the platform fallback.
///
/// Returns the rendered RGBA image or a combined backend and fallback error.
pub(super) fn snapshot_window_image(window: &mut Window) -> Result<image::RgbaImage, String> {
    match window.render_to_image() {
        Ok(image) => Ok(image),
        Err(err) => snapshot_window_image_fallback(window)
            .map_err(|fallback| format!("{err}; fallback failed: {fallback}")),
    }
}

/// Captures the visible gallery client area through Win32 GDI.
///
/// Returns an RGBA image or an error naming the failed Win32 capture step.
#[cfg(target_os = "windows")]
fn snapshot_window_image_fallback(window: &Window) -> Result<image::RgbaImage, String> {
    use windows::Win32::Foundation::{HWND, LPARAM, POINT};
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetClientRect, GetWindowThreadProcessId, HWND_NOTOPMOST,
        HWND_TOPMOST, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetCursorPos,
        SetForegroundWindow, SetWindowPos, ShowWindow,
    };

    /// Restores the captured gallery window to a non-topmost state on drop.
    struct TopmostGuard {
        hwnd: Option<HWND>,
    }

    impl Drop for TopmostGuard {
        fn drop(&mut self) {
            if let Some(hwnd) = self.hwnd {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_NOTOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                    );
                }
            }
        }
    }

    /// Carries the current process id and the first matching top-level window.
    struct EnumState {
        pid: u32,
        hwnd: Option<HWND>,
    }

    /// Records the first top-level window owned by the current process.
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let state = unsafe { &mut *(lparam.0 as *mut EnumState) };
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == state.pid {
            state.hwnd = Some(hwnd);
            return windows::core::BOOL(0);
        }
        windows::core::BOOL(1)
    }

    /// Finds the first top-level window owned by the gallery process.
    unsafe fn find_gallery_window() -> Option<HWND> {
        let mut state = EnumState {
            pid: unsafe { GetCurrentProcessId() },
            hwnd: None,
        };
        let state_ptr = &mut state as *mut EnumState;
        let _ = unsafe { EnumWindows(Some(enum_windows_proc), LPARAM(state_ptr as isize)) };
        state.hwnd
    }

    let mut topmost_guard = TopmostGuard { hwnd: None };
    let (x, y, width, height) = unsafe {
        match find_gallery_window() {
            Some(hwnd) => {
                topmost_guard.hwnd = Some(hwnd);
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                let _ = BringWindowToTop(hwnd);
                let _ = SetForegroundWindow(hwnd);

                let mut rect = Default::default();
                if GetClientRect(hwnd, &mut rect).is_err() {
                    return Err("GetClientRect failed".to_string());
                }
                let mut origin = POINT { x: 0, y: 0 };
                if !ClientToScreen(hwnd, &mut origin).as_bool() {
                    return Err("ClientToScreen failed".to_string());
                }
                let width = (rect.right - rect.left).max(1);
                let height = (rect.bottom - rect.top).max(1);

                // The Windows fallback captures real desktop pixels. Keep the
                // cursor away from the taskbar so thumbnail previews or other
                // shell overlays cannot be baked into component snapshots.
                let _ = SetCursorPos(origin.x + 8, origin.y + 8);
                std::thread::sleep(std::time::Duration::from_millis(350));
                (origin.x, origin.y, width, height)
            }
            None => {
                let bounds = window.bounds();
                (
                    f32::from(bounds.origin.x).round() as i32,
                    f32::from(bounds.origin.y).round() as i32,
                    f32::from(bounds.size.width).round().max(1.0) as i32,
                    f32::from(bounds.size.height).round().max(1.0) as i32,
                )
            }
        }
    };

    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("GetDC returned invalid HDC".to_string());
        }
        let memory = CreateCompatibleDC(Some(screen));
        if memory.is_invalid() {
            ReleaseDC(None, screen);
            return Err("CreateCompatibleDC returned invalid HDC".to_string());
        }
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            return Err("CreateCompatibleBitmap returned invalid HBITMAP".to_string());
        }

        let previous = SelectObject(memory, bitmap.into());
        let bitblt_ok = BitBlt(memory, 0, 0, width, height, Some(screen), x, y, SRCCOPY).is_ok();
        let _ = SelectObject(memory, previous);
        if !bitblt_ok {
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            return Err("BitBlt failed".to_string());
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0_u8; (width as usize) * (height as usize) * 4];
        let lines = GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );

        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);

        if lines == 0 {
            return Err("GetDIBits returned 0 lines".to_string());
        }

        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            .ok_or_else(|| "image::RgbaImage::from_raw failed".to_string())
    }
}

/// Reports that no platform capture fallback exists outside Windows.
#[cfg(not(target_os = "windows"))]
fn snapshot_window_image_fallback(_window: &Window) -> Result<image::RgbaImage, String> {
    Err("no platform fallback; implement backend render_to_image for this target".to_string())
}
