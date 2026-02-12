use super::context::{ini_default_filename, DearImguiContext, FIRST_MANAGED_TEXTURE_ID};
use super::extract::{
    dear_imgui_extract_frame_system, DearImguiExtractState, DearImguiRenderContext, NonSendHack,
};
use super::input::dear_imgui_new_frame_system;
use super::render::{DearImguiNode, DearImguiNodeLabel};
use super::snapshot::dear_imgui_end_frame_system;
use super::textures::dear_imgui_update_textures_system;
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::render::render_graph::RenderGraphExt;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{ExtractSchedule, Render, RenderApp, RenderSystems};
use bevy::window::PrimaryWindow;
use dear_imgui_rs::render::snapshot::TextureFeedback;
use dear_imgui_rs::{BackendFlags, ConfigFlags};
use dear_imgui_wgpu::{WgpuInitInfo, WgpuRenderer};
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use wgpu::TextureFormat;

/// Configuration settings for this plugin.
pub struct DearImguiPlugin {
    /// Sets the path to the ini file (default is "imgui.ini").
    /// Pass None to disable automatic .Ini saving.
    pub ini_filename: Option<PathBuf>,

    /// The config flags to supply to ImGui's IO when the context is initialized.
    pub config_flags: ConfigFlags,

    /// The unscaled font size to use (default is 13).
    pub font_size: f32,

    /// The number of horizontal font samples to perform. Must be >= 1 (default is 1).
    pub font_oversample_h: i32,

    /// The number of vertical font samples to perform. Must be >= 1 (default is 1).
    pub font_oversample_v: i32,

    /// Whether to apply the window display scale to the font size (default is true).
    pub apply_display_scale_to_font_size: bool,

    /// Whether to apply the window display scale to the number of font samples (default is true).
    pub apply_display_scale_to_font_oversample: bool,
}

impl Clone for DearImguiPlugin {
    fn clone(&self) -> Self {
        Self {
            ini_filename: self.ini_filename.clone(),
            config_flags: ConfigFlags::from_bits_truncate(self.config_flags.bits()),
            font_size: self.font_size,
            font_oversample_h: self.font_oversample_h,
            font_oversample_v: self.font_oversample_v,
            apply_display_scale_to_font_size: self.apply_display_scale_to_font_size,
            apply_display_scale_to_font_oversample: self.apply_display_scale_to_font_oversample,
        }
    }
}

#[cfg(not(feature = "docking"))]
fn default_config_flags() -> ConfigFlags {
    ConfigFlags::empty()
}

#[cfg(feature = "docking")]
fn default_config_flags() -> ConfigFlags {
    ConfigFlags::DOCKING_ENABLE
}

impl Default for DearImguiPlugin {
    fn default() -> Self {
        Self {
            ini_filename: ini_default_filename(),
            config_flags: default_config_flags(),
            font_size: 13.0,
            font_oversample_h: 1,
            font_oversample_v: 1,
            apply_display_scale_to_font_size: true,
            apply_display_scale_to_font_oversample: true,
        }
    }
}

impl Plugin for DearImguiPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let mut ctx = dear_imgui_rs::Context::create();
        ctx.set_ini_filename(self.ini_filename.clone()).unwrap();

        let io = ctx.io_mut();
        io.set_config_flags(ConfigFlags::from_bits_truncate(self.config_flags.bits()));
        let mut backend_flags = io.backend_flags();
        backend_flags.insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);
        backend_flags.insert(BackendFlags::RENDERER_HAS_TEXTURES);
        io.set_backend_flags(backend_flags);

        let display_scale = {
            let mut system_state: SystemState<Query<&Window, With<PrimaryWindow>>> =
                SystemState::new(app.world_mut());
            let primary_window = system_state.get(app.world());
            primary_window.single().unwrap().scale_factor()
        };

        // Bevy's primary surface is typically `Bgra8UnormSrgb`. If it differs at runtime,
        // the extract system will recreate the renderer with the actual swap chain format.
        let texture_format = TextureFormat::Bgra8UnormSrgb;

        let texture_feedback = Arc::new(Mutex::new(Vec::<TextureFeedback>::new()));

        let context = DearImguiContext {
            ctx: RwLock::new(ctx),
            plugin_settings: self.clone(),
            ui: None,
            textures: HashMap::new(),
            texture_modify: default(),
            managed_textures: HashMap::new(),
            managed_next_free_id: FIRST_MANAGED_TEXTURE_ID,
            texture_feedback: texture_feedback.clone(),
            extract_state: RwLock::new(DearImguiExtractState {
                next_frame_texture_format: texture_format,
                ..default()
            }),
            input_state: default(),
            last_display_scale: display_scale,
        };

        {
            let mut imgui_ctx = context.ctx.write().unwrap();
            update_display_scale(1.0, display_scale, &context.plugin_settings, &mut imgui_ctx);
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        {
            let mut system_state: SystemState<(Res<RenderDevice>, Res<RenderQueue>)> =
                SystemState::new(render_app.world_mut());
            let (device, queue) = system_state.get_mut(render_app.world_mut());

            let mut imgui_ctx = context.ctx.write().unwrap();
            let renderer = create_renderer(
                texture_format,
                device.as_ref(),
                queue.as_ref(),
                &mut imgui_ctx,
            );

            render_app.add_render_graph_node::<DearImguiNode>(Core2d, DearImguiNodeLabel);
            render_app.add_render_graph_edges(Core2d, (Node2d::EndMainPass, DearImguiNodeLabel));
            render_app.add_render_graph_edges(
                Core2d,
                (Node2d::EndMainPassPostProcessing, DearImguiNodeLabel),
            );
            render_app.add_render_graph_edges(Core2d, (Node2d::Upscaling, DearImguiNodeLabel));

            render_app.add_render_graph_node::<DearImguiNode>(Core3d, DearImguiNodeLabel);
            render_app.add_render_graph_edges(Core3d, (Node3d::EndMainPass, DearImguiNodeLabel));
            render_app.add_render_graph_edges(
                Core3d,
                (Node3d::EndMainPassPostProcessing, DearImguiNodeLabel),
            );
            render_app.add_render_graph_edges(Core3d, (Node3d::Upscaling, DearImguiNodeLabel));

            render_app.insert_resource(DearImguiRenderContext {
                renderer: RwLock::new(renderer),
                draw: default(),
                managed_texture_requests: Vec::new(),
                textures_to_add: HashMap::new(),
                textures_to_remove: Vec::new(),
                texture_feedback: texture_feedback.clone(),
            });

            render_app.world_mut().insert_non_send_resource(NonSendHack);
            render_app.add_systems(ExtractSchedule, dear_imgui_extract_frame_system);
            render_app.add_systems(
                Render,
                dear_imgui_update_textures_system.in_set(RenderSystems::Prepare),
            );
        }

        app.insert_non_send_resource(context);
        app.add_systems(PreUpdate, dear_imgui_new_frame_system);
        app.add_systems(Last, dear_imgui_end_frame_system);
    }
}

pub(crate) fn create_renderer(
    texture_format: TextureFormat,
    device: &RenderDevice,
    queue: &RenderQueue,
    imgui_ctx: &mut dear_imgui_rs::Context,
) -> WgpuRenderer {
    // Bevy wraps wgpu types in `WgpuWrapper<T>` to preserve Send/Sync behavior on wasm. We need the
    // underlying `wgpu::Queue` value here.
    let init_info = WgpuInitInfo::new(
        device.wgpu_device().clone(),
        queue.as_ref().deref().clone(),
        texture_format,
    );
    WgpuRenderer::new(init_info, imgui_ctx).unwrap()
}

pub(crate) fn update_display_scale(
    previous_display_scale: f32,
    display_scale: f32,
    settings: &DearImguiPlugin,
    imgui_ctx: &mut dear_imgui_rs::Context,
) {
    let font_scale = if settings.apply_display_scale_to_font_size {
        display_scale
    } else {
        1.0
    };
    let font_oversample_scale = if settings.apply_display_scale_to_font_oversample {
        display_scale
    } else {
        1.0
    };

    let oversample_h = ((settings.font_oversample_h as f32) * font_oversample_scale)
        .round()
        .clamp(1.0, i8::MAX as f32) as i8;
    let oversample_v = ((settings.font_oversample_v as f32) * font_oversample_scale)
        .round()
        .clamp(1.0, i8::MAX as f32) as i8;
    let font_size_px = f32::floor(settings.font_size * font_scale);

    {
        let io = imgui_ctx.io_mut();
        io.set_display_framebuffer_scale([display_scale, display_scale]);
    }

    // Dear ImGui 1.92+ uses style font scaling fields (FontScaleMain / FontScaleDpi).
    imgui_ctx.style_mut().set_font_scale_main(1.0 / font_scale);

    let mut atlas = imgui_ctx.font_atlas_mut();
    atlas.clear();
    let font_cfg = dear_imgui_rs::FontConfig::new()
        .size_pixels(font_size_px)
        .oversample_h(oversample_h)
        .oversample_v(oversample_v);
    atlas.add_font_default(Some(&font_cfg));

    unsafe {
        // Follow the Dear ImGui DPI scaling guidance (scale style sizes).
        dear_imgui_rs::sys::ImGuiStyle_ScaleAllSizes(
            dear_imgui_rs::sys::igGetStyle(),
            display_scale / previous_display_scale,
        );
    }
}
