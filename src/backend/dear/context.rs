use super::extract::DearImguiExtractState;
use super::input::DearImguiInputState;
use super::plugin::DearImguiPlugin;
use super::textures::DearImguiTextureModifyState;
use bevy::{asset::StrongHandle, prelude::*};
use dear_imgui_rs::render::snapshot::{ManagedTextureId, TextureFeedback};
use dear_imgui_rs::texture::TextureId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, RwLock};

pub(crate) const FIRST_USER_TEXTURE_ID: u64 = 1024;
pub(crate) const FIRST_MANAGED_TEXTURE_ID: u64 = 1;

/// Dear ImGui context resource.
pub struct DearImguiContext {
    pub(crate) ctx: RwLock<dear_imgui_rs::Context>,
    pub(crate) plugin_settings: DearImguiPlugin,
    pub(crate) ui: Option<NonNull<dear_imgui_rs::Ui>>,

    // User-registered Bevy textures (legacy TextureId path)
    pub(crate) textures: HashMap<TextureId, Arc<StrongHandle>>,
    pub(crate) texture_modify: RwLock<DearImguiTextureModifyState>,

    // ImGui-managed textures (ImGui 1.92+). We allocate stable legacy TextureIds so the render
    // thread can bind by id without carrying ImTextureData pointers across threads.
    pub(crate) managed_textures: HashMap<ManagedTextureId, TextureId>,
    pub(crate) managed_next_free_id: u64,

    // Cross-thread feedback queue for managed textures.
    pub(crate) texture_feedback: Arc<Mutex<Vec<TextureFeedback>>>,

    pub(crate) extract_state: RwLock<DearImguiExtractState>,
    pub(crate) input_state: DearImguiInputState,
    pub(crate) last_display_scale: f32,
}

impl DearImguiContext {
    pub fn ui(&mut self) -> &mut dear_imgui_rs::Ui {
        unsafe {
            self.ui
                .expect("Not currently rendering an imgui frame!")
                .as_mut()
        }
    }

    pub fn with_ui_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut dear_imgui_rs::Ui) -> R,
    {
        f(self.ui())
    }

    pub fn with_io_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut dear_imgui_rs::Io) -> R,
    {
        let mut ctx = self
            .ctx
            .write()
            .expect("Failed to acquire write access to Dear ImGui context");
        f(ctx.io_mut())
    }

    pub fn register_bevy_texture(&mut self, handle: Handle<Image>) -> TextureId {
        if let Handle::Strong(strong) = handle {
            let texture_modify = self.texture_modify.get_mut().unwrap();
            let next = &mut texture_modify.next_free_id;
            *next = u64::max(FIRST_USER_TEXTURE_ID, *next);

            let id = TextureId::new(*next);
            self.textures.insert(id, strong.clone());
            texture_modify.to_add.push(id);
            *next += 1;
            id
        } else {
            panic!("register_bevy_texture requires a strong Handle<Image>");
        }
    }

    pub fn unregister_bevy_texture(&mut self, texture_id: &TextureId) {
        self.textures.remove(texture_id);
        self.texture_modify
            .get_mut()
            .unwrap()
            .to_remove
            .push(*texture_id);
    }
}

pub(crate) fn ini_default_filename() -> Option<PathBuf> {
    Some(PathBuf::from("imgui.ini"))
}
