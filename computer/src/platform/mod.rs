pub mod types;
pub mod keymap_common;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{input, outputs, screenshot, windows};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{input, outputs, screenshot, windows};
