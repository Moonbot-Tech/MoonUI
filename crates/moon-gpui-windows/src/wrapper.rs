use std::{
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HCURSOR};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SafeCursor {
    raw: HCURSOR,
}

unsafe impl Send for SafeCursor {}
unsafe impl Sync for SafeCursor {}

impl From<HCURSOR> for SafeCursor {
    fn from(value: HCURSOR) -> Self {
        SafeCursor { raw: value }
    }
}

impl Deref for SafeCursor {
    type Target = HCURSOR;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SafeHwnd {
    raw: HWND,
}

impl SafeHwnd {
    pub(crate) fn as_raw(&self) -> HWND {
        self.raw
    }
}

unsafe impl Send for SafeHwnd {}
unsafe impl Sync for SafeHwnd {}

impl From<HWND> for SafeHwnd {
    fn from(value: HWND) -> Self {
        SafeHwnd { raw: value }
    }
}

impl Deref for SafeHwnd {
    type Target = HWND;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FrameClockWindow {
    hwnd: SafeHwnd,
    frame_clock_pending: Arc<AtomicBool>,
}

impl FrameClockWindow {
    pub(crate) fn new(hwnd: HWND, frame_clock_pending: Arc<AtomicBool>) -> Self {
        Self {
            hwnd: hwnd.into(),
            frame_clock_pending,
        }
    }

    pub(crate) fn as_raw(&self) -> HWND {
        self.hwnd.as_raw()
    }

    pub(crate) fn try_mark_frame_clock_pending(&self) -> bool {
        self.frame_clock_pending
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub(crate) fn clear_frame_clock_pending(&self) {
        self.frame_clock_pending.store(false, Ordering::Release);
    }
}
