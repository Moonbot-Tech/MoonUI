//! Minimal native surface owner for renderer tests, without a GPUI app or an added dependency.

#[cfg(windows)]
mod platform {
    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
        RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
    };
    use std::{ffi::c_void, num::NonZeroIsize, sync::Arc};

    #[link(name = "user32")]
    unsafe extern "system" {
        fn CreateWindowExW(
            ex_style: u32,
            class: *const u16,
            name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: *mut c_void,
            menu: *mut c_void,
            instance: *mut c_void,
            param: *mut c_void,
        ) -> *mut c_void;
        fn DestroyWindow(window: *mut c_void) -> i32;
    }

    /// Keep the HWND alive through all clones retained by the GPU instance.
    #[derive(Clone, Debug)]
    pub struct TestWindow(Arc<Owner>);

    #[derive(Debug)]
    struct Owner(usize);

    impl TestWindow {
        pub fn new() -> Self {
            let class: Vec<_> = "STATIC\0".encode_utf16().collect();
            // A visible non-activating tool window also works on drivers that reject hidden surfaces.
            let window = unsafe {
                CreateWindowExW(
                    0x08000080,
                    class.as_ptr(),
                    class.as_ptr(),
                    0x90000000,
                    0,
                    0,
                    64,
                    64,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            assert!(!window.is_null(), "create renderer test window");
            Self(Arc::new(Owner(window as usize)))
        }
    }

    impl Drop for Owner {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0 as *mut c_void);
            }
        }
    }

    impl HasWindowHandle for TestWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let handle = Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).unwrap());
            Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
        }
    }

    impl HasDisplayHandle for TestWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(unsafe {
                DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
            })
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
        RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle,
    };
    use std::{
        ffi::{c_char, c_int, c_ulong, c_void},
        ptr::NonNull,
        sync::Arc,
    };

    #[link(name = "X11")]
    unsafe extern "C" {
        fn XOpenDisplay(name: *const c_char) -> *mut c_void;
        fn XDefaultScreen(display: *mut c_void) -> c_int;
        fn XRootWindow(display: *mut c_void, screen: c_int) -> c_ulong;
        fn XCreateSimpleWindow(
            display: *mut c_void,
            parent: c_ulong,
            x: c_int,
            y: c_int,
            width: u32,
            height: u32,
            border_width: u32,
            border: c_ulong,
            background: c_ulong,
        ) -> c_ulong;
        fn XMapWindow(display: *mut c_void, window: c_ulong) -> c_int;
        fn XFlush(display: *mut c_void) -> c_int;
        fn XDestroyWindow(display: *mut c_void, window: c_ulong) -> c_int;
        fn XCloseDisplay(display: *mut c_void) -> c_int;
    }

    /// Keep the Xlib connection and window alive through the GPU instance's window clone.
    #[derive(Clone, Debug)]
    pub struct TestWindow(Arc<Owner>);

    #[derive(Debug)]
    struct Owner {
        display: usize,
        screen: i32,
        window: c_ulong,
    }

    impl TestWindow {
        pub fn new() -> Self {
            let display = unsafe { XOpenDisplay(std::ptr::null()) };
            assert!(
                !display.is_null(),
                "renderer test requires DISPLAY (xvfb-run is supported)"
            );
            let screen = unsafe { XDefaultScreen(display) };
            let window = unsafe {
                XCreateSimpleWindow(display, XRootWindow(display, screen), 0, 0, 64, 64, 0, 0, 0)
            };
            unsafe {
                XMapWindow(display, window);
                XFlush(display);
            }
            Self(Arc::new(Owner {
                display: display as usize,
                screen,
                window,
            }))
        }
    }

    impl Drop for Owner {
        fn drop(&mut self) {
            unsafe {
                XDestroyWindow(self.display as *mut c_void, self.window);
                XCloseDisplay(self.display as *mut c_void);
            }
        }
    }

    impl HasWindowHandle for TestWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            Ok(unsafe {
                WindowHandle::borrow_raw(RawWindowHandle::Xlib(XlibWindowHandle::new(
                    self.0.window,
                )))
            })
        }
    }

    impl HasDisplayHandle for TestWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            let handle =
                XlibDisplayHandle::new(NonNull::new(self.0.display as *mut c_void), self.0.screen);
            Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Xlib(handle)) })
        }
    }
}

pub use platform::TestWindow;
