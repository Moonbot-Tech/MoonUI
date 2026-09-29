use std::{ops::Deref, sync::Arc};

use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::HCURSOR};

use crate::FrameClockState;

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
    frame_clock: Arc<FrameClockState>,
}

impl FrameClockWindow {
    pub(crate) fn new(hwnd: HWND, frame_clock: Arc<FrameClockState>) -> Self {
        Self {
            hwnd: hwnd.into(),
            frame_clock,
        }
    }

    pub(crate) fn as_raw(&self) -> HWND {
        self.hwnd.as_raw()
    }

    pub(crate) fn frame_clock(&self) -> &FrameClockState {
        &self.frame_clock
    }
}
