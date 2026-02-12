use dear_imgui_rs::render::snapshot::{
    DrawCmdSnapshot, FrameSnapshot, ManagedTextureId, TextureBinding,
};
use dear_imgui_rs::render::snapshot::{SnapshotOptions, UserCallbackPolicy};
use dear_imgui_rs::render::DrawData;
use dear_imgui_rs::sys;
use dear_imgui_rs::texture::TextureId;
use std::collections::HashMap;

use super::context::{DearImguiContext, FIRST_USER_TEXTURE_ID};
use super::extract::ThreadedTextureRequest;
use bevy::prelude::*;

#[derive(Default)]
pub(crate) struct OwnedDrawDataSend {
    draw_data: *mut sys::ImDrawData,
}

unsafe impl Send for OwnedDrawDataSend {}
unsafe impl Sync for OwnedDrawDataSend {}

impl OwnedDrawDataSend {
    pub(crate) fn draw_data(&self) -> Option<&DrawData> {
        if self.draw_data.is_null() {
            None
        } else {
            Some(unsafe { &*(self.draw_data as *const DrawData) })
        }
    }

    pub(crate) fn patch_textures_from_snapshot(
        &mut self,
        snapshot: &FrameSnapshot,
        managed_textures: &HashMap<ManagedTextureId, TextureId>,
    ) {
        if self.draw_data.is_null() {
            return;
        }

        unsafe {
            let draw_data = &mut *self.draw_data;
            let cmd_lists_count = usize::try_from(draw_data.CmdListsCount).unwrap_or(0);

            if draw_data.CmdLists.Data.is_null() {
                return;
            }

            for list_i in 0..cmd_lists_count {
                let list_ptr = *draw_data.CmdLists.Data.add(list_i);
                if list_ptr.is_null() {
                    continue;
                }

                let list = &mut *list_ptr;
                if list.CmdBuffer.Data.is_null() {
                    continue;
                }

                let Some(snapshot_list) = snapshot.draw.draw_lists.get(list_i) else {
                    continue;
                };

                let cmd_count = usize::try_from(list.CmdBuffer.Size).unwrap_or(0);
                let patch_count = cmd_count.min(snapshot_list.commands.len());
                let cmds = std::slice::from_raw_parts_mut(list.CmdBuffer.Data, patch_count);

                for (cmd_i, snapshot_cmd) in
                    snapshot_list.commands.iter().take(patch_count).enumerate()
                {
                    // Always clear any pointers to ImGui-managed global state in the render copy.
                    cmds[cmd_i].TexRef._TexData = std::ptr::null_mut();
                    cmds[cmd_i].TexRef._TexID = 0;

                    let DrawCmdSnapshot::Elements { texture, .. } = snapshot_cmd else {
                        continue;
                    };

                    let tex_id = match texture {
                        TextureBinding::Legacy(tex_id) => *tex_id,
                        TextureBinding::Managed(managed_id) => managed_textures
                            .get(managed_id)
                            .copied()
                            .unwrap_or_else(TextureId::null),
                    };

                    cmds[cmd_i].TexRef._TexID = tex_id.id();
                }
            }
        }
    }
}

impl From<&DrawData> for OwnedDrawDataSend {
    fn from(value: &DrawData) -> Self {
        unsafe fn fixup_cloned_draw_list(draw_list: *mut sys::ImDrawList) {
            if draw_list.is_null() {
                return;
            }

            // ImDrawList::CloneOutput() clones Cmd/Idx/Vtx buffers but leaves internal write pointers
            // (and current index) in an invalid state. ImDrawData::AddDrawList() performs a debug
            // sanity check that these are consistent.
            let draw_list = &mut *draw_list;

            draw_list._VtxCurrentIdx = u32::try_from(draw_list.VtxBuffer.Size).unwrap_or(0);

            draw_list._VtxWritePtr = if draw_list.VtxBuffer.Data.is_null() {
                std::ptr::null_mut()
            } else {
                draw_list
                    .VtxBuffer
                    .Data
                    .add(draw_list.VtxBuffer.Size.max(0) as usize)
            };

            draw_list._IdxWritePtr = if draw_list.IdxBuffer.Data.is_null() {
                std::ptr::null_mut()
            } else {
                draw_list
                    .IdxBuffer
                    .Data
                    .add(draw_list.IdxBuffer.Size.max(0) as usize)
            };
        }

        unsafe {
            let result = sys::ImDrawData_ImDrawData();
            if result.is_null() {
                panic!("Failed to allocate ImDrawData for OwnedDrawDataSend");
            }

            let source_ptr = value as *const DrawData as *const sys::ImDrawData;

            (*result).Valid = (*source_ptr).Valid;
            (*result).TotalIdxCount = (*source_ptr).TotalIdxCount;
            (*result).TotalVtxCount = (*source_ptr).TotalVtxCount;
            (*result).DisplayPos = (*source_ptr).DisplayPos;
            (*result).DisplaySize = (*source_ptr).DisplaySize;
            (*result).FramebufferScale = (*source_ptr).FramebufferScale;

            // Do not keep pointers to ImGui-managed global state in the render copy.
            (*result).OwnerViewport = std::ptr::null_mut();
            (*result).Textures = std::ptr::null_mut();

            (*result).CmdListsCount = 0;
            if (*source_ptr).CmdListsCount > 0 && !(*source_ptr).CmdLists.Data.is_null() {
                for i in 0..((*source_ptr).CmdListsCount as usize) {
                    let src_list = *(*source_ptr).CmdLists.Data.add(i);
                    if src_list.is_null() {
                        continue;
                    }
                    let cloned = sys::ImDrawList_CloneOutput(src_list);
                    if !cloned.is_null() {
                        fixup_cloned_draw_list(cloned);
                        sys::ImDrawData_AddDrawList(result, cloned);
                    }
                }
            }

            Self { draw_data: result }
        }
    }
}

impl Drop for OwnedDrawDataSend {
    fn drop(&mut self) {
        unsafe {
            if self.draw_data.is_null() {
                return;
            }

            if !(*self.draw_data).CmdLists.Data.is_null() {
                for i in 0..((*self.draw_data).CmdListsCount as usize) {
                    let ptr = *(*self.draw_data).CmdLists.Data.add(i);
                    if !ptr.is_null() {
                        sys::ImDrawList_destroy(ptr);
                    }
                }
            }

            sys::ImDrawData_destroy(self.draw_data);
            self.draw_data = std::ptr::null_mut();
        }
    }
}

#[derive(Default)]
pub(crate) struct OwnedDrawDataSendWrap(pub(crate) OwnedDrawDataSend);

pub(crate) fn dear_imgui_end_frame_system(mut context: NonSendMut<DearImguiContext>) {
    // End the frame by rendering it to generate draw data. We do this outside of extract to keep
    // the extract path cheap (extract blocks both the game and render thread).
    let context = context.as_mut();
    let draw_data = context.ctx.get_mut().unwrap().render();
    context.ui = None;

    let snapshot = match FrameSnapshot::from_draw_data(
        draw_data,
        SnapshotOptions {
            user_callback_policy: UserCallbackPolicy::Error,
            capture_texture_requests: true,
        },
    ) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            log::error!("Failed to snapshot Dear ImGui draw data; skipping frame: {err:?}");
            let extract_state = context.extract_state.get_mut().unwrap();
            extract_state.rendered_draw_data = default();
            extract_state.rendered_texture_requests.clear();
            return;
        }
    };

    // Allocate stable legacy TextureIds for managed textures so we can:
    // - patch cloned draw data to avoid raw ImTextureData pointers across threads
    // - let the render thread create/update/destroy GPU textures by TextureId
    {
        let managed_textures = &mut context.managed_textures;
        let next_free_id = &mut context.managed_next_free_id;

        let mut alloc = || {
            let id = *next_free_id;
            *next_free_id = next_free_id.saturating_add(1);
            debug_assert!(
                id < FIRST_USER_TEXTURE_ID,
                "Managed TextureId space exhausted (collides with user textures)"
            );
            TextureId::new(id)
        };

        for draw_list in &snapshot.draw.draw_lists {
            for cmd in &draw_list.commands {
                let DrawCmdSnapshot::Elements { texture, .. } = cmd else {
                    continue;
                };
                if let TextureBinding::Managed(managed_id) = texture {
                    managed_textures
                        .entry(*managed_id)
                        .or_insert_with(&mut alloc);
                }
            }
        }
        for req in &snapshot.texture_requests {
            managed_textures.entry(req.id).or_insert_with(&mut alloc);
        }
    }

    let mut threaded_requests =
        Vec::<ThreadedTextureRequest>::with_capacity(snapshot.texture_requests.len());
    for req in &snapshot.texture_requests {
        let tex_id = context
            .managed_textures
            .get(&req.id)
            .copied()
            .unwrap_or_else(TextureId::null);
        threaded_requests.push(ThreadedTextureRequest {
            managed_id: req.id,
            tex_id,
            op: req.op.clone(),
        });
    }

    let mut owned_draw_data = OwnedDrawDataSend::from(draw_data);
    owned_draw_data.patch_textures_from_snapshot(&snapshot, &context.managed_textures);

    let extract_state = context.extract_state.get_mut().unwrap();
    extract_state.rendered_draw_data = owned_draw_data;
    extract_state.rendered_texture_requests = threaded_requests;
}
