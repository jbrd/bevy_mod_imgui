//! A Dear ImGui integration for the Bevy game engine.
//!
//! # Minimal Example
//!
//! ```rust
//! use bevy::prelude::*;
//! use bevy_mod_imgui::prelude::*;
//!
//! #[derive(Resource)]
//! struct ImguiState {
//!     demo_window_open: bool,
//! }
//!
//! fn main() {
//!     let mut app = App::new();
//!     app.insert_resource(ClearColor(Color::rgba(0.2, 0.2, 0.2, 1.0)))
//!         .insert_resource(ImguiState {
//!             demo_window_open: true,
//!         })
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
//!         .add_systems(Startup, |mut commands: Commands| {
//!             commands.spawn(Camera3dBundle::default());
//!         })
//!         .add_systems(Update, imgui_example_ui);
//!     app.run();
//! }
//!
//! fn imgui_example_ui(mut context: NonSendMut<ImguiContext>, mut state: ResMut<ImguiState>) {
//!     let ui = context.ui();
//!     if state.demo_window_open {
//!         ui.show_demo_window(&mut state.demo_window_open);
//!     }
//! }
//! ```

use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ExtractedWindows,
        RenderApp,
    },
    window::{PrimaryWindow, WindowScaleFactorChanged},
};
use imgui::{FontSource, OwnedDrawData};
use imgui_wgpu::{Renderer, RendererConfig};
use std::{
    cell::RefCell,
    path::PathBuf,
    ptr::null_mut,
    sync::{Arc, Mutex},
};
use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, TextureFormat,
};

/// The ImGui context resource.
///
/// This should be added to your Bevy app as a `NonSendMut` resource (as it is not thread safe).
///
/// You can use this object to obtain a reference to the underlying `imgui::Ui` object for submitting
/// UI elements to imgui. This should be done during the Update and PostUpdate phase only.
pub struct ImguiContext {
    ctx: Arc<Mutex<imgui::Context>>,
    ui: *mut imgui::Ui,
    display_scale: f32,
    font_scale: bool,
}

impl ImguiContext {
    /// Provides mutable access to the underlying `imgui::Ui` object.
    ///
    /// Use this to submit UI elements to imgui.
    pub fn ui(&mut self) -> &mut imgui::Ui {
        unsafe { &mut *self.ui }
    }
}

#[derive(Resource)]
struct ImguiRenderContext {
    ctx: Arc<Mutex<imgui::Context>>,
    renderer: RefCell<Renderer>,
    texture_format: TextureFormat,
    draw: imgui::OwnedDrawData,
}

unsafe impl Send for ImguiRenderContext {}
unsafe impl Sync for ImguiRenderContext {}

struct ImGuiNode;

impl ImGuiNode {
    fn create_render_pass<'a>(
        command_encoder: &'a mut CommandEncoder,
        world: &'a World,
    ) -> Result<RenderPass<'a>, ()> {
        let extracted_windows = &world.get_resource::<ExtractedWindows>().unwrap();
        let Some(primary) = extracted_windows.primary else {
            return Err(()); // No primary window
        };
        let extracted_window = &extracted_windows.windows[&primary];

        let swap_chain_texture_view = if let Some(swap_chain_texture_view) =
            extracted_window.swap_chain_texture_view.as_ref()
        {
            swap_chain_texture_view
        } else {
            return Err(()); // No swapchain texture
        };

        Ok(command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: swap_chain_texture_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        }))
    }
}

impl Node for ImGuiNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let context = world.resource::<ImguiRenderContext>();
        let queue = world.get_resource::<RenderQueue>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let command_encoder = render_context.command_encoder();
        let wgpu_device = render_device.wgpu_device();
        let mut renderer = context.renderer.borrow_mut();
        if let Ok(mut rpass) = ImGuiNode::create_render_pass(command_encoder, world) {
            if let Some(draw_data) = context.draw.draw_data() {
                renderer
                    .render(draw_data, queue, wgpu_device, &mut rpass)
                    .unwrap();
            }
        }
        Ok(())
    }
}

/// Configuration settings for this plugin.
pub struct ImguiPlugin {
    /// Sets the path to the ini file (default is "imgui.ini").
    /// Pass None to disable automatic .Ini saving
    pub ini_filename: Option<PathBuf>,

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

impl Default for ImguiPlugin {
    fn default() -> Self {
        Self {
            ini_filename: Default::default(),
            font_size: 13.0,
            font_oversample_h: 1,
            font_oversample_v: 1,
            apply_display_scale_to_font_size: true,
            apply_display_scale_to_font_oversample: true,
        }
    }
}

impl Plugin for ImguiPlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        let display_scale = {
            let mut system_state: SystemState<Query<&Window, With<PrimaryWindow>>> =
                SystemState::new(&mut app.world);
            let primary_window = system_state.get(&app.world);
            primary_window.get_single().unwrap().scale_factor() as f32
        };

        let font_scale = if self.apply_display_scale_to_font_size {
            display_scale
        } else {
            1.0
        };

        let font_oversample_scale = if self.apply_display_scale_to_font_oversample {
            display_scale.ceil() as i32
        } else {
            1
        };

        let mut ctx = imgui::Context::create();
        ctx.set_ini_filename(self.ini_filename.clone());
        ctx.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: self.font_size * font_scale,
                oversample_h: self.font_oversample_h * font_oversample_scale,
                oversample_v: self.font_oversample_v * font_oversample_scale,
                ..default()
            }),
        }]);

        for key_index in 0..imgui::Key::COUNT {
            ctx.io_mut()[imgui::Key::VARIANTS[key_index]] = key_index as _;
        }

        let ctx_arc = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => {
                let device = render_app.world.resource::<RenderDevice>();
                let queue = render_app.world.resource::<RenderQueue>();

                // Here we create a new ImGui renderer with a default format. At this point,
                // we don't know what format the window surface is going to be set up with,
                // and yet we need to initialise the renderer so that the texture glyphs
                // are created before new_frame is called on the imgui context.
                //
                // This will give us a functonal imgui context. If, at the point at which we
                // extract the scene, we realise that the window has an incompatible
                // format, the renderer will be recreated with a compatible format.
                let renderer_config = RendererConfig::default();
                let texture_format = renderer_config.texture_format;
                let renderer =
                    Renderer::new(&mut ctx, device.wgpu_device(), queue, renderer_config);

                let mut graph = render_app.world.resource_mut::<RenderGraph>();

                if let Some(graph_2d) =
                    graph.get_sub_graph_mut(bevy::core_pipeline::core_2d::graph::NAME)
                {
                    let imgui_node = ImGuiNode;
                    graph_2d.add_node("imgui", imgui_node);
                    graph_2d.add_node_edge(
                        bevy::core_pipeline::core_2d::graph::node::MAIN_PASS,
                        "imgui",
                    );
                    graph_2d.add_node_edge(
                        bevy::core_pipeline::core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                        "imgui",
                    );
                    graph_2d.add_node_edge(
                        bevy::core_pipeline::core_2d::graph::node::UPSCALING,
                        "imgui",
                    );
                }

                if let Some(graph_3d) =
                    graph.get_sub_graph_mut(bevy::core_pipeline::core_3d::graph::NAME)
                {
                    let imgui_node = ImGuiNode;
                    graph_3d.add_node("imgui", imgui_node);
                    graph_3d.add_node_edge(
                        bevy::core_pipeline::core_3d::graph::node::END_MAIN_PASS,
                        "imgui",
                    );
                    graph_3d.add_node_edge(
                        bevy::core_pipeline::core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                        "imgui",
                    );
                    graph_3d.add_node_edge(
                        bevy::core_pipeline::core_3d::graph::node::UPSCALING,
                        "imgui",
                    );
                }

                let arc = Arc::new(Mutex::new(ctx));
                render_app.insert_resource(ImguiRenderContext {
                    ctx: arc.clone(),
                    renderer: RefCell::new(renderer),
                    texture_format,
                    draw: OwnedDrawData::default(),
                });

                render_app.add_systems(ExtractSchedule, imgui_extract_frame_system);

                arc
            }
            _ => {
                return;
            }
        };

        app.insert_non_send_resource(ImguiContext {
            ctx: ctx_arc,
            ui: null_mut(),
            display_scale,
            font_scale: self.apply_display_scale_to_font_size,
        });

        app.add_systems(PreUpdate, imgui_new_frame_system);
    }
}

fn imgui_new_frame_system(
    mut context: NonSendMut<ImguiContext>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<bevy::input::mouse::MouseButton>>,
    mut received_chars: EventReader<ReceivedCharacter>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut scale_events: EventReader<WindowScaleFactorChanged>,
) {
    const IMGUI_TO_BEVY_KEYS: [bevy::input::keyboard::KeyCode; imgui::Key::COUNT] = [
        KeyCode::Tab,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::Back,
        KeyCode::Space,
        KeyCode::Return,
        KeyCode::Escape,
        KeyCode::ControlLeft,
        KeyCode::ShiftLeft,
        KeyCode::AltLeft,
        KeyCode::SuperLeft,
        KeyCode::ControlRight,
        KeyCode::ShiftRight,
        KeyCode::AltRight,
        KeyCode::SuperRight,
        KeyCode::Apps, // sys::ImGuiKey_Menu
        KeyCode::Key0,
        KeyCode::Key1,
        KeyCode::Key2,
        KeyCode::Key3,
        KeyCode::Key4,
        KeyCode::Key5,
        KeyCode::Key6,
        KeyCode::Key7,
        KeyCode::Key8,
        KeyCode::Key9,
        KeyCode::A,
        KeyCode::B,
        KeyCode::C,
        KeyCode::D,
        KeyCode::E,
        KeyCode::F,
        KeyCode::G,
        KeyCode::H,
        KeyCode::I,
        KeyCode::J,
        KeyCode::K,
        KeyCode::L,
        KeyCode::M,
        KeyCode::N,
        KeyCode::O,
        KeyCode::P,
        KeyCode::Q,
        KeyCode::R,
        KeyCode::S,
        KeyCode::T,
        KeyCode::U,
        KeyCode::V,
        KeyCode::W,
        KeyCode::X,
        KeyCode::Y,
        KeyCode::Z,
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
        KeyCode::Apostrophe,
        KeyCode::Comma,
        KeyCode::Minus,
        KeyCode::Period,
        KeyCode::Slash,
        KeyCode::Semicolon,
        KeyCode::Equals,
        KeyCode::BracketLeft,
        KeyCode::Backslash,
        KeyCode::BracketRight,
        KeyCode::Grave,
        KeyCode::Capital,
        KeyCode::Scroll,
        KeyCode::Numlock,
        KeyCode::Snapshot,
        KeyCode::Pause,
        KeyCode::Numpad0,
        KeyCode::Numpad1,
        KeyCode::Numpad2,
        KeyCode::Numpad3,
        KeyCode::Numpad4,
        KeyCode::Numpad5,
        KeyCode::Numpad6,
        KeyCode::Numpad7,
        KeyCode::Numpad8,
        KeyCode::Numpad9,
        KeyCode::NumpadDecimal,
        KeyCode::NumpadDivide,
        KeyCode::NumpadMultiply,
        KeyCode::NumpadSubtract,
        KeyCode::NumpadAdd,
        KeyCode::NumpadEnter,
        KeyCode::NumpadEquals,
        KeyCode::Unlabeled, // GamepadStart = sys::ImGuiKey_GamepadStart,
        KeyCode::Unlabeled, // GamepadBack = sys::ImGuiKey_GamepadBack,
        KeyCode::Unlabeled, // GamepadFaceLeft = sys::ImGuiKey_GamepadFaceLeft,
        KeyCode::Unlabeled, // GamepadFaceRight = sys::ImGuiKey_GamepadFaceRight,
        KeyCode::Unlabeled, // GamepadFaceUp = sys::ImGuiKey_GamepadFaceUp,
        KeyCode::Unlabeled, // GamepadFaceDown = sys::ImGuiKey_GamepadFaceDown,
        KeyCode::Unlabeled, // GamepadDpadLeft = sys::ImGuiKey_GamepadDpadLeft,
        KeyCode::Unlabeled, // GamepadDpadRight = sys::ImGuiKey_GamepadDpadRight,
        KeyCode::Unlabeled, // GamepadDpadUp = sys::ImGuiKey_GamepadDpadUp,
        KeyCode::Unlabeled, // GamepadDpadDown = sys::ImGuiKey_GamepadDpadDown,
        KeyCode::Unlabeled, // GamepadL1 = sys::ImGuiKey_GamepadL1,
        KeyCode::Unlabeled, // GamepadR1 = sys::ImGuiKey_GamepadR1,
        KeyCode::Unlabeled, // GamepadL2 = sys::ImGuiKey_GamepadL2,
        KeyCode::Unlabeled, // GamepadR2 = sys::ImGuiKey_GamepadR2,
        KeyCode::Unlabeled, // GamepadL3 = sys::ImGuiKey_GamepadL3,
        KeyCode::Unlabeled, // GamepadR3 = sys::ImGuiKey_GamepadR3,
        KeyCode::Unlabeled, // GamepadLStickLeft = sys::ImGuiKey_GamepadLStickLeft,
        KeyCode::Unlabeled, // GamepadLStickRight = sys::ImGuiKey_GamepadLStickRight,
        KeyCode::Unlabeled, // GamepadLStickUp = sys::ImGuiKey_GamepadLStickUp,
        KeyCode::Unlabeled, // GamepadLStickDown = sys::ImGuiKey_GamepadLStickDown,
        KeyCode::Unlabeled, // GamepadRStickLeft = sys::ImGuiKey_GamepadRStickLeft,
        KeyCode::Unlabeled, // GamepadRStickRight = sys::ImGuiKey_GamepadRStickRight,
        KeyCode::Unlabeled, // GamepadRStickUp = sys::ImGuiKey_GamepadRStickUp,
        KeyCode::Unlabeled, // GamepadRStickDown = sys::ImGuiKey_GamepadRStickDown,
        KeyCode::Unlabeled, // MouseLeft = sys::ImGuiKey_MouseLeft,
        KeyCode::Unlabeled, // MouseRight = sys::ImGuiKey_MouseRight,
        KeyCode::Unlabeled, // MouseMiddle = sys::ImGuiKey_MouseMiddle,
        KeyCode::Unlabeled, // MouseX1 = sys::ImGuiKey_MouseX1,
        KeyCode::Unlabeled, // MouseX2 = sys::ImGuiKey_MouseX2,
        KeyCode::Unlabeled, // MouseWheelX = sys::ImGuiKey_MouseWheelX,
        KeyCode::Unlabeled, // MouseWheelY = sys::ImGuiKey_MouseWheelY,
        KeyCode::Unlabeled, // ReservedForModCtrl = sys::ImGuiKey_ReservedForModCtrl,
        KeyCode::Unlabeled, // ReservedForModShift = sys::ImGuiKey_ReservedForModShift,
        KeyCode::Unlabeled, // ReservedForModAlt = sys::ImGuiKey_ReservedForModAlt,
        KeyCode::Unlabeled, // ReservedForModSuper = sys::ImGuiKey_ReservedForModSuper
    ];

    for WindowScaleFactorChanged {
        window,
        scale_factor,
    } in scale_events.iter()
    {
        if primary_window.get_single().unwrap().0 == *window {
            context.display_scale = *scale_factor as f32;
        }
    }

    let ui_ptr: *mut imgui::Ui;
    let display_scale = context.display_scale;
    let font_scale = context.font_scale;
    {
        let mut ctx = context.ctx.lock().unwrap();
        let io = ctx.io_mut();

        let Ok((_, primary)) = primary_window.get_single() else {
            return;
        };

        io.display_size = [primary.width(), primary.height()];
        io.display_framebuffer_scale = [display_scale, display_scale];
        io.font_global_scale = if font_scale { 1.0 / display_scale } else { 1.0 };

        if let Some(pos) = primary.cursor_position() {
            io.mouse_pos = [pos.x, pos.y];
        }

        io.mouse_down[0] = mouse.pressed(bevy::input::mouse::MouseButton::Left);
        io.mouse_down[1] = mouse.pressed(bevy::input::mouse::MouseButton::Right);
        io.mouse_down[2] = mouse.pressed(bevy::input::mouse::MouseButton::Middle);

        for e in received_chars.iter() {
            io.add_input_character(e.char);
        }

        for (key_index, key) in IMGUI_TO_BEVY_KEYS.iter().enumerate() {
            io.keys_down[key_index] = keyboard.pressed(*key);
        }

        io.key_alt = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);
        io.key_ctrl =
            keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
        io.key_shift =
            keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        io.key_super =
            keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight);

        for e in mouse_wheel.iter() {
            io.mouse_wheel = e.y;
            io.mouse_wheel_h = e.x;
        }
        ui_ptr = ctx.new_frame();
    }
    context.ui = ui_ptr;
}

fn imgui_extract_frame_system(
    mut context: ResMut<ImguiRenderContext>,
    extracted_windows: ResMut<ExtractedWindows>,
    device: Res<RenderDevice>,
    queue: ResMut<RenderQueue>,
) {
    // End the imgui frame.
    let owned_draw_data = {
        let mut ctx = context.ctx.lock().unwrap();
        let draw_data = ctx.render();
        OwnedDrawData::from(draw_data)
    };

    // We've now recorded the draw data for the current frame, and this should be renderer agnostic.
    // So at this point, we can check to see whether the texture format of the target window matches
    // the renderer's texture format. If it doesn't, we recreate the Renderer here before we proceed
    // to render the frame.
    context.draw = OwnedDrawData::default();

    let Some(primary) = extracted_windows.primary else {
        return;
    };
    let extracted_window = &extracted_windows.windows[&primary];
    let Some(texture_format) = extracted_window.swap_chain_texture_format else {
        return;
    };
    if texture_format != context.texture_format {
        let renderer_config = RendererConfig {
            texture_format,
            ..default()
        };
        context.renderer.swap(&RefCell::new(Renderer::new(
            &mut context.ctx.lock().unwrap(),
            device.wgpu_device(),
            &queue,
            renderer_config,
        )));
        context.texture_format = texture_format;
    }
    context.draw = owned_draw_data;
}

pub mod prelude {
    pub use crate::*;
    pub use imgui::*;
}
