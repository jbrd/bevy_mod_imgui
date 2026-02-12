//! A Dear ImGui integration for the Bevy game engine.
//!
//! This crate supports two mutually-exclusive backends:
//!
//! - `imgui-rs` (default): based on the `imgui` crate.
//! - `dear-imgui-rs`: based on the `dear-imgui-rs` crate.
//!
//! To use the `dear-imgui-rs` backend, disable default features and enable `dear-imgui-rs`:
//!
//! ```text
//! cargo run --example dear-minimal --no-default-features --features dear-imgui-rs
//! ```

#[cfg(all(feature = "imgui-rs", feature = "dear-imgui-rs"))]
compile_error!("Features `imgui-rs` and `dear-imgui-rs` are mutually exclusive. Use `--no-default-features --features dear-imgui-rs` to switch backends.");

#[cfg(not(any(feature = "imgui-rs", feature = "dear-imgui-rs")))]
compile_error!("No backend selected. Enable default features (imgui-rs) or use `--no-default-features --features dear-imgui-rs`.");

mod backend;

#[cfg(feature = "imgui-rs")]
pub use backend::imgui::*;

#[cfg(feature = "dear-imgui-rs")]
pub use backend::dear::*;

#[cfg(feature = "dear-imgui-rs")]
pub mod dear {
    pub use crate::backend::dear::*;
}

mod renderer;
