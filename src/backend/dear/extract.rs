use super::context::DearImguiContext;
use super::plugin::create_renderer;
use super::snapshot::{OwnedDrawDataSend, OwnedDrawDataSendWrap};
use bevy::asset::StrongHandle;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::view::ExtractedWindows;
use bevy::render::Extract;
use bevy::window::PrimaryWindow;
use dear_imgui_rs::render::snapshot::{ManagedTextureId, TextureFeedback, TextureOp};
use dear_imgui_rs::texture::TextureId;
use dear_imgui_wgpu::WgpuRenderer;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::TextureFormat;

pub(crate) struct DearImguiExtractState {
    pub(crate) rendered_draw_data: OwnedDrawDataSend,
    pub(crate) rendered_texture_requests: Vec<ThreadedTextureRequest>,
    pub(crate) next_frame_renderer: Option<WgpuRenderer>,
    pub(crate) next_frame_texture_format: TextureFormat,
}

impl Default for DearImguiExtractState {
    fn default() -> Self {
        Self {
            rendered_draw_data: default(),
            rendered_texture_requests: Vec::new(),
            next_frame_renderer: None,
            next_frame_texture_format: TextureFormat::Bgra8UnormSrgb,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ThreadedTextureRequest {
    pub(crate) managed_id: ManagedTextureId,
    pub(crate) tex_id: TextureId,
    pub(crate) op: TextureOp,
}

/// Used to force a system to be `NonSend`, due to `Extract<NonSend<T>>` not working.
#[allow(dead_code)]
pub(crate) struct NonSendHack;

#[derive(Resource)]
pub(crate) struct DearImguiRenderContext {
    pub(crate) renderer: RwLock<WgpuRenderer>,
    pub(crate) draw: OwnedDrawDataSendWrap,
    pub(crate) managed_texture_requests: Vec<ThreadedTextureRequest>,
    pub(crate) textures_to_add: HashMap<TextureId, (Arc<StrongHandle>, ImageSampler)>,
    pub(crate) textures_to_remove: Vec<TextureId>,
    pub(crate) texture_feedback: Arc<Mutex<Vec<TextureFeedback>>>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dear_imgui_extract_frame_system(
    mut imgui_context: Extract<NonSend<DearImguiContext>>,
    mut render_context: ResMut<DearImguiRenderContext>,
    extracted_windows: ResMut<ExtractedWindows>,
    device: Res<RenderDevice>,
    queue: ResMut<RenderQueue>,
    images: Extract<Res<Assets<Image>>>,
    _non_send: NonSend<NonSendHack>,
    _primary_window: Extract<Query<&Window, With<PrimaryWindow>>>,
) {
    let imgui_context = imgui_context.deref_mut();
    let mut extract_state = imgui_context.extract_state.write().unwrap();

    let owned_draw_data = std::mem::take(&mut extract_state.rendered_draw_data);
    let texture_requests = std::mem::take(&mut extract_state.rendered_texture_requests);

    // If the renderer was re-created for this frame, pass it into the render context now.
    if extract_state.next_frame_renderer.is_some() {
        render_context.renderer = RwLock::new(extract_state.next_frame_renderer.take().unwrap());

        // Re-add all user-registered textures (legacy TextureId path).
        for (texture_id, reference) in imgui_context.textures.iter() {
            let sampler = images
                .get(&Handle::<Image>::Strong(reference.clone()))
                .map_or(ImageSampler::Default, |x| x.sampler.clone());
            render_context
                .textures_to_add
                .insert(*texture_id, (reference.clone(), sampler));
        }
    }

    // Determine the current texture format of the primary window.
    let Some(primary) = extracted_windows.primary else {
        return;
    };
    let Some(extracted_window) = extracted_windows.windows.get(&primary) else {
        return;
    };
    let Some(texture_format) = extracted_window.swap_chain_texture_format else {
        return;
    };

    // Only submit draw data if it matches the renderer's expected target format.
    if texture_format == extract_state.next_frame_texture_format {
        render_context.draw = OwnedDrawDataSendWrap(owned_draw_data);
    } else {
        render_context.draw = default();
    }

    render_context.managed_texture_requests = texture_requests;

    // Recreate renderer if the surface format changed.
    if texture_format != extract_state.next_frame_texture_format {
        let mut ctx = imgui_context
            .ctx
            .write()
            .expect("Failed to acquire write access to Dear ImGui context");
        let next = create_renderer(texture_format, device.as_ref(), queue.as_ref(), &mut ctx);
        extract_state.next_frame_renderer = Some(next);
        extract_state.next_frame_texture_format = texture_format;
    }

    // Submit texture modifications (user-registered Bevy textures).
    for texture_id in imgui_context.texture_modify.read().unwrap().to_add.iter() {
        let reference = imgui_context.textures[texture_id].clone();
        let sampler = images
            .get(&Handle::<Image>::Strong(reference.clone()))
            .map_or(ImageSampler::Default, |x| x.sampler.clone());
        render_context
            .textures_to_add
            .insert(*texture_id, (reference, sampler));
    }

    let mut texture_modify = imgui_context.texture_modify.write().unwrap();
    render_context
        .textures_to_remove
        .clone_from(&texture_modify.to_remove);
    texture_modify.to_add.clear();
    texture_modify.to_remove.clear();
}
