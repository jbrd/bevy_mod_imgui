use super::extract::DearImguiRenderContext;
use bevy::prelude::*;
use bevy::render::render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel};
use bevy::render::renderer::RenderContext;
use bevy::render::view::ExtractedWindows;
use std::sync::RwLockWriteGuard;
use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp,
};

/// The label used by the render node responsible for rendering Dear ImGui.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct DearImguiNodeLabel;

pub(crate) struct DearImguiNode;

impl DearImguiNode {
    fn create_render_pass<'a>(
        command_encoder: &'a mut CommandEncoder,
        world: &'a World,
    ) -> Result<RenderPass<'a>, ()> {
        let extracted_windows = &world.get_resource::<ExtractedWindows>().unwrap();
        let Some(primary) = extracted_windows.primary else {
            return Err(());
        };
        let Some(extracted_window) = extracted_windows.windows.get(&primary) else {
            return Err(());
        };
        let Some(swap_chain_texture_view) = extracted_window.swap_chain_texture_view.as_ref()
        else {
            return Err(());
        };

        Ok(command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: swap_chain_texture_view,
                resolve_target: None,
                depth_slice: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        }))
    }

    fn renderer<'a>(
        imgui_render_context: &'a DearImguiRenderContext,
    ) -> RwLockWriteGuard<'a, dear_imgui_wgpu::WgpuRenderer> {
        imgui_render_context.renderer.write().unwrap()
    }
}

impl Node for DearImguiNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let imgui_render_context = world.resource::<DearImguiRenderContext>();
        let command_encoder = render_context.command_encoder();

        let mut renderer = Self::renderer(imgui_render_context);
        if let Ok(mut rpass) = Self::create_render_pass(command_encoder, world) {
            if let Some(draw_data) = imgui_render_context.draw.0.draw_data() {
                renderer.new_frame().unwrap();
                renderer.render_draw_data(draw_data, &mut rpass).unwrap();
            }
        }

        Ok(())
    }
}

impl FromWorld for DearImguiNode {
    fn from_world(_world: &mut World) -> DearImguiNode {
        DearImguiNode {}
    }
}
