//! A Dear ImGui integration for the Bevy game engine.
//!
//! # Minimal Example
//!
//! ```no_run
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
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::system::SystemState,
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel},
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::ExtractedWindows,
        RenderApp,
    },
    window::{PrimaryWindow, WindowScaleFactorChanged},
};
use imgui::{FontSource, OwnedDrawData};
mod imgui_wgpu_rs_local;
use imgui_wgpu_rs_local::{Renderer, RendererConfig};
use std::{cell::RefCell, path::PathBuf, ptr::null_mut, rc::Rc, sync::Mutex};
use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureFormat,
};

/// The ImGui context resource.
///
/// This should be added to your Bevy app as a `NonSendMut` resource (as it is not thread safe).
///
/// You can use this object to obtain a reference to the underlying `imgui::Ui` object for submitting
/// UI elements to imgui. This should be done during the Update and PostUpdate phase only.
pub struct ImguiContext {
    ctx: Rc<Mutex<imgui::Context>>,
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
    ctx: Rc<Mutex<imgui::Context>>,
    renderer: RefCell<Renderer>,
    texture_format: TextureFormat,
    draw: imgui::OwnedDrawData,
}

unsafe impl Send for ImguiRenderContext {}
unsafe impl Sync for ImguiRenderContext {}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ImGuiNodeLabel;

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
        let Some(extracted_window) = extracted_windows.windows.get(&primary) else {
            return Err(()); // No primary window
        };
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
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
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

impl FromWorld for ImGuiNode {
    fn from_world(_world: &mut World) -> ImGuiNode {
        ImGuiNode {}
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
            primary_window.get_single().unwrap().scale_factor()
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

        let ctx_rc = match app.get_sub_app_mut(RenderApp) {
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

                render_app.add_render_graph_node::<ImGuiNode>(Core2d, ImGuiNodeLabel);

                render_app.add_render_graph_edges(Core2d, (Node2d::MainPass, ImGuiNodeLabel));

                render_app.add_render_graph_edges(
                    Core2d,
                    (Node2d::EndMainPassPostProcessing, ImGuiNodeLabel),
                );

                render_app.add_render_graph_edges(Core2d, (Node2d::Upscaling, ImGuiNodeLabel));

                render_app.add_render_graph_node::<ImGuiNode>(Core3d, ImGuiNodeLabel);

                render_app.add_render_graph_edges(Core3d, (Node3d::EndMainPass, ImGuiNodeLabel));

                render_app.add_render_graph_edges(
                    Core3d,
                    (Node3d::EndMainPassPostProcessing, ImGuiNodeLabel),
                );

                render_app.add_render_graph_edges(Core3d, (Node3d::Upscaling, ImGuiNodeLabel));

                let rc = Rc::new(Mutex::new(ctx));
                render_app.insert_resource(ImguiRenderContext {
                    ctx: rc.clone(),
                    renderer: RefCell::new(renderer),
                    texture_format,
                    draw: OwnedDrawData::default(),
                });

                render_app.add_systems(ExtractSchedule, imgui_extract_frame_system);

                rc
            }
            _ => {
                return;
            }
        };

        app.insert_non_send_resource(ImguiContext {
            ctx: ctx_rc,
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
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    mut received_chars: EventReader<ReceivedCharacter>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut scale_events: EventReader<WindowScaleFactorChanged>,
) {
    const UNKNOWN_KEYCODE: KeyCode = KeyCode::F35;
    const IMGUI_TO_BEVY_KEYS: [bevy::input::keyboard::KeyCode; imgui::Key::COUNT] = [
        KeyCode::Tab,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Insert,
        KeyCode::Delete,
        KeyCode::Backspace,
        KeyCode::Space,
        KeyCode::Enter,
        KeyCode::Escape,
        KeyCode::ControlLeft,
        KeyCode::ShiftLeft,
        KeyCode::AltLeft,
        KeyCode::SuperLeft,
        KeyCode::ControlRight,
        KeyCode::ShiftRight,
        KeyCode::AltRight,
        KeyCode::SuperRight,
        KeyCode::ContextMenu, // sys::ImGuiKey_Menu
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
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
        KeyCode::Quote,
        KeyCode::Comma,
        KeyCode::Minus,
        KeyCode::Period,
        KeyCode::Slash,
        KeyCode::Semicolon,
        KeyCode::Equal,
        KeyCode::BracketLeft,
        KeyCode::Backslash,
        KeyCode::BracketRight,
        KeyCode::Backquote,
        KeyCode::CapsLock,
        KeyCode::ScrollLock,
        KeyCode::NumLock,
        KeyCode::PrintScreen,
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
        KeyCode::NumpadEqual,
        UNKNOWN_KEYCODE, // GamepadStart = sys::ImGuiKey_GamepadStart,
        UNKNOWN_KEYCODE, // GamepadBack = sys::ImGuiKey_GamepadBack,
        UNKNOWN_KEYCODE, // GamepadFaceLeft = sys::ImGuiKey_GamepadFaceLeft,
        UNKNOWN_KEYCODE, // GamepadFaceRight = sys::ImGuiKey_GamepadFaceRight,
        UNKNOWN_KEYCODE, // GamepadFaceUp = sys::ImGuiKey_GamepadFaceUp,
        UNKNOWN_KEYCODE, // GamepadFaceDown = sys::ImGuiKey_GamepadFaceDown,
        UNKNOWN_KEYCODE, // GamepadDpadLeft = sys::ImGuiKey_GamepadDpadLeft,
        UNKNOWN_KEYCODE, // GamepadDpadRight = sys::ImGuiKey_GamepadDpadRight,
        UNKNOWN_KEYCODE, // GamepadDpadUp = sys::ImGuiKey_GamepadDpadUp,
        UNKNOWN_KEYCODE, // GamepadDpadDown = sys::ImGuiKey_GamepadDpadDown,
        UNKNOWN_KEYCODE, // GamepadL1 = sys::ImGuiKey_GamepadL1,
        UNKNOWN_KEYCODE, // GamepadR1 = sys::ImGuiKey_GamepadR1,
        UNKNOWN_KEYCODE, // GamepadL2 = sys::ImGuiKey_GamepadL2,
        UNKNOWN_KEYCODE, // GamepadR2 = sys::ImGuiKey_GamepadR2,
        UNKNOWN_KEYCODE, // GamepadL3 = sys::ImGuiKey_GamepadL3,
        UNKNOWN_KEYCODE, // GamepadR3 = sys::ImGuiKey_GamepadR3,
        UNKNOWN_KEYCODE, // GamepadLStickLeft = sys::ImGuiKey_GamepadLStickLeft,
        UNKNOWN_KEYCODE, // GamepadLStickRight = sys::ImGuiKey_GamepadLStickRight,
        UNKNOWN_KEYCODE, // GamepadLStickUp = sys::ImGuiKey_GamepadLStickUp,
        UNKNOWN_KEYCODE, // GamepadLStickDown = sys::ImGuiKey_GamepadLStickDown,
        UNKNOWN_KEYCODE, // GamepadRStickLeft = sys::ImGuiKey_GamepadRStickLeft,
        UNKNOWN_KEYCODE, // GamepadRStickRight = sys::ImGuiKey_GamepadRStickRight,
        UNKNOWN_KEYCODE, // GamepadRStickUp = sys::ImGuiKey_GamepadRStickUp,
        UNKNOWN_KEYCODE, // GamepadRStickDown = sys::ImGuiKey_GamepadRStickDown,
        UNKNOWN_KEYCODE, // MouseLeft = sys::ImGuiKey_MouseLeft,
        UNKNOWN_KEYCODE, // MouseRight = sys::ImGuiKey_MouseRight,
        UNKNOWN_KEYCODE, // MouseMiddle = sys::ImGuiKey_MouseMiddle,
        UNKNOWN_KEYCODE, // MouseX1 = sys::ImGuiKey_MouseX1,
        UNKNOWN_KEYCODE, // MouseX2 = sys::ImGuiKey_MouseX2,
        UNKNOWN_KEYCODE, // MouseWheelX = sys::ImGuiKey_MouseWheelX,
        UNKNOWN_KEYCODE, // MouseWheelY = sys::ImGuiKey_MouseWheelY,
        UNKNOWN_KEYCODE, // ReservedForModCtrl = sys::ImGuiKey_ReservedForModCtrl,
        UNKNOWN_KEYCODE, // ReservedForModShift = sys::ImGuiKey_ReservedForModShift,
        UNKNOWN_KEYCODE, // ReservedForModAlt = sys::ImGuiKey_ReservedForModAlt,
        UNKNOWN_KEYCODE, // ReservedForModSuper = sys::ImGuiKey_ReservedForModSuper
    ];

    for WindowScaleFactorChanged {
        window,
        scale_factor,
    } in scale_events.read()
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

        for e in received_chars.read() {
            io.add_input_character(e.char.chars().last().unwrap());
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

        for e in mouse_wheel.read() {
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
    let Some(extracted_window) = extracted_windows.windows.get(&primary) else {
        return;
    };
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
