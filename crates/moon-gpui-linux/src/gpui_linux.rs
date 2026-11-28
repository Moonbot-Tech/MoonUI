#![cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

pub use linux::current_platform;

/// The application's choice of GPU and present mode for the Linux renderer.
#[cfg(any(feature = "wayland", feature = "x11"))]
pub use gpui_wgpu::gpu_selection;
