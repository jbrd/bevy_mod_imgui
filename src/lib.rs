use bevy::{
    prelude::*,
    render::{
        render_graph::{Node, RenderGraph},
        view::ExtractedWindows,
        RenderApp, RenderStage,
    },
    window::WindowId,
};
use imgui::FontSource;
use imgui_wgpu::Renderer;
use std::{
    cell::RefCell,
    path::PathBuf,
    ptr::null_mut,
    sync::{Arc, Mutex},
};
use wgpu::RenderPassDescriptor;

pub struct ImguiContext {
    ctx: Arc<Mutex<imgui::Context>>,
    ui: *mut imgui::Ui,
    display_scale: f32,
    font_scale: bool,
}

impl ImguiContext {
    pub fn ui(&mut self) -> &mut imgui::Ui {
        unsafe { &mut *self.ui }
    }
}

struct ImguiRenderContext {
    ctx: Arc<Mutex<imgui::Context>>,
    renderer: RefCell<Renderer>,
    draw: *const imgui::DrawData,
}

struct ImGuiNode;

impl Node for ImGuiNode {
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let context = world.non_send_resource::<ImguiRenderContext>();
        let mut renderer = context.renderer.borrow_mut();

        let device = &render_context.render_device.wgpu_device();
        let queue = world
            .get_resource::<bevy::render::renderer::RenderQueue>()
            .unwrap();

        let extracted_windows = &world.get_resource::<ExtractedWindows>().unwrap().windows;
        let extracted_window =
            if let Some(extracted_window) = extracted_windows.get(&WindowId::primary()) {
                extracted_window
            } else {
                return Ok(()); // No window
            };

        let swap_chain_texture =
            if let Some(swap_chain_texture) = extracted_window.swap_chain_texture.as_ref() {
                swap_chain_texture
            } else {
                return Ok(()); // No swapchain texture
            };

        let mut rpass = render_context
            .command_encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: swap_chain_texture,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

        renderer
            .render(unsafe { &*context.draw }, queue, device, &mut rpass)
            .unwrap();

        Ok(())
    }
}

pub struct ImguiPlugin {
    pub ini_filename: Option<PathBuf>,
    pub font_size: f32,
    pub font_oversample_h: i32,
    pub font_oversample_v: i32,
    pub apply_display_scale_to_font_size: bool,
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
    fn build(&self, app: &mut App) {
        let device = app.world.resource::<bevy::render::renderer::RenderDevice>();
        let queue = app.world.resource::<bevy::render::renderer::RenderQueue>();
        let display_scale = app.world.resource::<Windows>().primary().scale_factor() as f32;

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

        let renderer = Renderer::new(
            &mut ctx,
            device.wgpu_device(),
            queue,
            imgui_wgpu::RendererConfig {
                texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
                ..default()
            },
        );

        let ctx_arc = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => {
                let mut graph = render_app.world.resource_mut::<RenderGraph>();

                if let Some(graph_2d) =
                    graph.get_sub_graph_mut(bevy::core_pipeline::core_2d::graph::NAME)
                {
                    let imgui_node = ImGuiNode;
                    graph_2d.add_node("imgui", imgui_node);
                    graph_2d
                        .add_node_edge(
                            bevy::core_pipeline::core_2d::graph::node::MAIN_PASS,
                            "imgui",
                        )
                        .unwrap();
                    graph_2d.add_node_edge(
                        bevy::core_pipeline::core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                        "imgui",
                    ).unwrap();
                    graph_2d
                        .add_node_edge(
                            bevy::core_pipeline::core_2d::graph::node::UPSCALING,
                            "imgui",
                        )
                        .unwrap();
                }

                if let Some(graph_3d) =
                    graph.get_sub_graph_mut(bevy::core_pipeline::core_3d::graph::NAME)
                {
                    let imgui_node = ImGuiNode;
                    graph_3d.add_node("imgui", imgui_node);
                    graph_3d
                        .add_node_edge(
                            bevy::core_pipeline::core_3d::graph::node::MAIN_PASS,
                            "imgui",
                        )
                        .unwrap();
                    graph_3d.add_node_edge(
                        bevy::core_pipeline::core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                        "imgui",
                    ).unwrap();
                    graph_3d
                        .add_node_edge(
                            bevy::core_pipeline::core_3d::graph::node::UPSCALING,
                            "imgui",
                        )
                        .unwrap();
                }

                let arc = Arc::new(Mutex::new(ctx));
                render_app.insert_non_send_resource(ImguiRenderContext {
                    ctx: arc.clone(),
                    renderer: RefCell::new(renderer),
                    draw: std::ptr::null(),
                });
                render_app.add_system_to_stage(RenderStage::Extract, imgui_extract_frame_system);

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

        app.add_system_to_stage(CoreStage::PreUpdate, imgui_new_frame_system.at_end());
    }
}

fn imgui_new_frame_system(
    mut context: NonSendMut<ImguiContext>,
    windows: Res<Windows>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<bevy::input::mouse::MouseButton>>,
    mut received_chars: EventReader<ReceivedCharacter>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
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
        KeyCode::NumpadEnter,
        KeyCode::A,
        KeyCode::C,
        KeyCode::V,
        KeyCode::X,
        KeyCode::Y,
        KeyCode::Z,
    ];

    if let Some(primary_window) = windows.get_primary() {
        let ui_ptr: *mut imgui::Ui;
        let display_scale = context.display_scale;
        let font_scale = context.font_scale;
        {
            let mut ctx = context.ctx.lock().unwrap();
            let mut io = ctx.io_mut();

            io.display_size = [primary_window.width(), primary_window.height()];
            io.display_framebuffer_scale = [display_scale, display_scale];
            io.font_global_scale = if font_scale { 1.0 / display_scale } else { 1.0 };

            if let Some(pos) = primary_window.cursor_position() {
                io.mouse_pos = [pos.x, primary_window.height() - pos.y];
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

            io.key_alt = keyboard.pressed(KeyCode::LAlt) || keyboard.pressed(KeyCode::RAlt);
            io.key_ctrl =
                keyboard.pressed(KeyCode::LControl) || keyboard.pressed(KeyCode::RControl);
            io.key_shift = keyboard.pressed(KeyCode::LShift) || keyboard.pressed(KeyCode::RShift);
            io.key_super = keyboard.pressed(KeyCode::LWin) || keyboard.pressed(KeyCode::RWin);

            for e in mouse_wheel.iter() {
                io.mouse_wheel = e.y;
                io.mouse_wheel_h = e.x;
            }
            ui_ptr = ctx.new_frame();
        }
        context.ui = ui_ptr;
    };
}

fn imgui_extract_frame_system(mut context: NonSendMut<ImguiRenderContext>) {
    let draw_ptr: *const imgui::DrawData;
    {
        let mut ctx = context.ctx.lock().unwrap();
        draw_ptr = ctx.render();
    }
    context.draw = draw_ptr;
}

pub mod prelude {
    pub use crate::*;
    pub use imgui::*;
}
