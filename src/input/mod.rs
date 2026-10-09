#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::InputHandler;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::InputHandler;