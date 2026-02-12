//! Dear ImGui integration based on `dear-imgui-rs`.
//!
//! Enable with the `dear-imgui-rs` feature, then add [`DearImguiPlugin`].
//!
//! This module is only compiled when the `dear-imgui-rs` feature is enabled.

mod context;
mod extract;
mod input;
mod plugin;
mod render;
mod snapshot;
mod textures;

pub use context::*;
pub use plugin::*;
pub use render::*;

pub mod prelude {
    pub use super::*;
    pub use dear_imgui_rs::*;
}
