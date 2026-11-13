#![cfg(any(target_os = "linux", target_os = "freebsd"))]
mod linux;

#[cfg(any(feature = "wayland", feature = "x11"))]
pub use linux::foreign_parent;
pub use linux::{current_platform, linux_platform};
