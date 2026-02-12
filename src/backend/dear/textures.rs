use super::extract::{DearImguiRenderContext, ThreadedTextureRequest};
use bevy::asset::StrongHandle;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::GpuImage;
use dear_imgui_rs::render::snapshot::{TextureFeedback, TextureOp, TextureUploadRect};
use dear_imgui_rs::texture::{TextureFormat as DearTextureFormat, TextureId, TextureRect};
use dear_imgui_wgpu::{WgpuRenderer, WgpuTexture};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct DearImguiTextureModifyState {
    pub(crate) to_add: Vec<TextureId>,
    pub(crate) to_remove: Vec<TextureId>,
    pub(crate) next_free_id: u64,
}

pub(crate) fn dear_imgui_update_textures_system(
    mut render_context: ResMut<DearImguiRenderContext>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let render_context = render_context.as_mut();

    // Remove textures.
    let textures_to_remove = std::mem::take(&mut render_context.textures_to_remove);
    for texture_id in textures_to_remove {
        let mut renderer = render_context.renderer.write().unwrap();
        renderer.unregister_texture(texture_id.id());
        render_context.textures_to_add.remove(&texture_id);
    }

    // Add (or refresh) external textures.
    let mut added = Vec::<TextureId>::new();
    for (texture_id, (handle, sampler)) in &render_context.textures_to_add {
        let mut renderer = render_context.renderer.write().unwrap();
        add_image_to_renderer(
            texture_id,
            handle,
            &gpu_images,
            sampler,
            &mut renderer,
            &device,
        );
        added.push(*texture_id);
    }
    for id in added {
        render_context.textures_to_add.remove(&id);
    }

    // Handle ImGui-managed textures (threaded snapshot + feedback path).
    let requests = std::mem::take(&mut render_context.managed_texture_requests);
    if !requests.is_empty() {
        let mut renderer = render_context.renderer.write().unwrap();
        let mut feedback_out = Vec::<TextureFeedback>::new();
        for req in &requests {
            handle_managed_texture_request(
                req,
                &mut renderer,
                device.wgpu_device(),
                queue.as_ref(),
                &mut feedback_out,
            );
        }

        if !feedback_out.is_empty() {
            let mut queue = render_context
                .texture_feedback
                .lock()
                .expect("Failed to lock Dear ImGui texture feedback queue");
            queue.extend(feedback_out);
        }
    }
}

fn add_image_to_renderer(
    texture_id: &TextureId,
    strong: &Arc<StrongHandle>,
    gpu_images: &RenderAssets<GpuImage>,
    sampler: &ImageSampler,
    renderer: &mut WgpuRenderer,
    device: &RenderDevice,
) {
    let handle = Handle::<Image>::Strong(strong.clone());
    let Some(gpu_image) = gpu_images.get(&handle) else {
        panic!("Could not obtain GPU image for texture. Please ensure textures are loaded prior to registering them with imgui");
    };

    let id = texture_id.id();

    renderer.unregister_texture(id);
    renderer.texture_manager_mut().insert_texture_with_id(
        id,
        WgpuTexture::new(
            gpu_image.texture.deref().clone(),
            gpu_image.texture_view.deref().clone(),
        ),
    );

    if let ImageSampler::Descriptor(desc) = sampler {
        let wgpu_sampler = device
            .wgpu_device()
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Bevy Texture Sampler for Dear ImGui"),
                address_mode_u: desc.address_mode_u.into(),
                address_mode_v: desc.address_mode_v.into(),
                address_mode_w: desc.address_mode_w.into(),
                mag_filter: desc.mag_filter.into(),
                min_filter: desc.min_filter.into(),
                mipmap_filter: desc.mipmap_filter.into(),
                lod_min_clamp: desc.lod_min_clamp,
                lod_max_clamp: desc.lod_max_clamp,
                compare: desc.compare.map(Into::into),
                anisotropy_clamp: desc.anisotropy_clamp,
                border_color: desc.border_color.map(Into::into),
            });
        renderer.update_external_texture_sampler(id, &wgpu_sampler);
    }
}

fn handle_managed_texture_request(
    req: &ThreadedTextureRequest,
    renderer: &mut WgpuRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    feedback: &mut Vec<TextureFeedback>,
) {
    match &req.op {
        TextureOp::Create {
            format,
            width,
            height,
            row_pitch,
            pixels,
        } => {
            if *width <= 0 || *height <= 0 {
                return;
            }

            let width_u32 = u32::try_from(*width).unwrap_or(0);
            let height_u32 = u32::try_from(*height).unwrap_or(0);
            if width_u32 == 0 || height_u32 == 0 {
                return;
            }

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Dear ImGui managed texture"),
                size: wgpu::Extent3d {
                    width: width_u32,
                    height: height_u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            renderer.unregister_texture(req.tex_id.id());
            renderer
                .texture_manager_mut()
                .insert_texture_with_id(req.tex_id.id(), WgpuTexture::new(texture, view));

            let Some(wgpu_tex) = renderer.texture_manager().get_texture(req.tex_id.id()) else {
                return;
            };

            let rgba = match format {
                DearTextureFormat::RGBA32 => pixels.clone(),
                DearTextureFormat::Alpha8 => alpha8_to_rgba8(pixels),
            };
            let bytes_per_row = match format {
                DearTextureFormat::RGBA32 => u32::try_from(*row_pitch).unwrap_or(0),
                DearTextureFormat::Alpha8 => width_u32.saturating_mul(4),
            };

            write_texture_rgba8(
                queue,
                wgpu_tex.texture(),
                wgpu::Origin3d::ZERO,
                width_u32,
                height_u32,
                &rgba,
                bytes_per_row,
            );

            feedback.push(TextureFeedback {
                id: req.managed_id,
                status: dear_imgui_rs::texture::TextureStatus::OK,
                tex_id: Some(req.tex_id),
            });
        }
        TextureOp::Update {
            format,
            width,
            height,
            rects,
        } => {
            if *width <= 0 || *height <= 0 {
                return;
            }

            let width_u32 = u32::try_from(*width).unwrap_or(0);
            let height_u32 = u32::try_from(*height).unwrap_or(0);
            if width_u32 == 0 || height_u32 == 0 {
                return;
            }

            let exists = renderer
                .texture_manager()
                .get_texture(req.tex_id.id())
                .is_some();
            if !exists {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Dear ImGui managed texture (late create)"),
                    size: wgpu::Extent3d {
                        width: width_u32,
                        height: height_u32,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                renderer.unregister_texture(req.tex_id.id());
                renderer
                    .texture_manager_mut()
                    .insert_texture_with_id(req.tex_id.id(), WgpuTexture::new(texture, view));
            }

            let Some(wgpu_tex) = renderer.texture_manager().get_texture(req.tex_id.id()) else {
                return;
            };

            for TextureUploadRect {
                rect,
                row_pitch,
                data,
            } in rects
            {
                let Some((rgba, bytes_per_row)) =
                    upload_rect_to_rgba8(*format, *rect, *row_pitch, data)
                else {
                    continue;
                };
                write_texture_rgba8(
                    queue,
                    wgpu_tex.texture(),
                    wgpu::Origin3d {
                        x: u32::from(rect.x),
                        y: u32::from(rect.y),
                        z: 0,
                    },
                    u32::from(rect.w),
                    u32::from(rect.h),
                    &rgba,
                    bytes_per_row,
                );
            }

            feedback.push(TextureFeedback {
                id: req.managed_id,
                status: dear_imgui_rs::texture::TextureStatus::OK,
                tex_id: Some(req.tex_id),
            });
        }
        TextureOp::Destroy => {
            renderer.unregister_texture(req.tex_id.id());
            feedback.push(TextureFeedback {
                id: req.managed_id,
                status: dear_imgui_rs::texture::TextureStatus::Destroyed,
                tex_id: None,
            });
        }
    }
}

fn alpha8_to_rgba8(pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len().saturating_mul(4));
    for &a in pixels {
        out.extend_from_slice(&[255, 255, 255, a]);
    }
    out
}

fn upload_rect_to_rgba8(
    format: DearTextureFormat,
    rect: TextureRect,
    row_pitch: i32,
    data: &[u8],
) -> Option<(Vec<u8>, u32)> {
    let w = usize::from(rect.w);
    let h = usize::from(rect.h);
    if w == 0 || h == 0 {
        return None;
    }

    match format {
        DearTextureFormat::RGBA32 => {
            let bytes_per_row = u32::try_from(row_pitch).ok()?;
            Some((data.to_vec(), bytes_per_row))
        }
        DearTextureFormat::Alpha8 => {
            let bytes_per_row = u32::try_from(w).ok()?.saturating_mul(4);
            let src_pitch = usize::try_from(row_pitch).ok()?;
            if src_pitch == 0 {
                return None;
            }
            let mut out = vec![0u8; w.saturating_mul(h).saturating_mul(4)];
            for row in 0..h {
                let src_off = row.saturating_mul(src_pitch);
                let dst_off = row.saturating_mul(w).saturating_mul(4);
                if src_off.saturating_add(w) > data.len()
                    || dst_off.saturating_add(w * 4) > out.len()
                {
                    return None;
                }
                for i in 0..w {
                    let a = data[src_off + i];
                    let dst = dst_off + i * 4;
                    out[dst..dst + 4].copy_from_slice(&[255, 255, 255, a]);
                }
            }
            Some((out, bytes_per_row))
        }
    }
}

fn write_texture_rgba8(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    origin: wgpu::Origin3d,
    width: u32,
    height: u32,
    rgba: &[u8],
    bytes_per_row: u32,
) {
    if width == 0 || height == 0 || bytes_per_row == 0 {
        return;
    }

    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = bytes_per_row.div_ceil(align) * align;

    if padded_bytes_per_row == bytes_per_row {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        return;
    }

    let unpadded = bytes_per_row as usize;
    let padded = padded_bytes_per_row as usize;
    let height_usize = height as usize;
    if unpadded == 0 || padded == 0 || height_usize == 0 {
        return;
    }

    let needed_unpadded = unpadded.saturating_mul(height_usize);
    if rgba.len() < needed_unpadded {
        return;
    }

    let mut padded_buf = vec![0u8; padded.saturating_mul(height_usize)];
    for row in 0..height_usize {
        let src_off = row.saturating_mul(unpadded);
        let dst_off = row.saturating_mul(padded);
        padded_buf[dst_off..dst_off + unpadded].copy_from_slice(&rgba[src_off..src_off + unpadded]);
    }

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        &padded_buf,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(padded_bytes_per_row),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}
