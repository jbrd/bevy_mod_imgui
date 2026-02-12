use super::context::DearImguiContext;
use super::plugin::update_display_scale;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use dear_imgui_rs::render::snapshot::TextureFeedback;
use dear_imgui_rs::texture::TextureStatus;
use std::ptr::NonNull;

#[derive(Default)]
pub(crate) struct DearImguiInputState {
    pub(crate) keys: Vec<bool>,
    pub(crate) mod_ctrl: bool,
    pub(crate) mod_shift: bool,
    pub(crate) mod_alt: bool,
    pub(crate) mod_super: bool,
    pub(crate) mouse_left: bool,
    pub(crate) mouse_right: bool,
    pub(crate) mouse_middle: bool,
}

pub(crate) fn key_map() -> &'static [(dear_imgui_rs::Key, bevy::input::keyboard::KeyCode)] {
    use bevy::input::keyboard::KeyCode as B;
    use dear_imgui_rs::Key as I;

    &[
        (I::Tab, B::Tab),
        (I::LeftArrow, B::ArrowLeft),
        (I::RightArrow, B::ArrowRight),
        (I::UpArrow, B::ArrowUp),
        (I::DownArrow, B::ArrowDown),
        (I::PageUp, B::PageUp),
        (I::PageDown, B::PageDown),
        (I::Home, B::Home),
        (I::End, B::End),
        (I::Insert, B::Insert),
        (I::Delete, B::Delete),
        (I::Backspace, B::Backspace),
        (I::Space, B::Space),
        (I::Enter, B::Enter),
        (I::Escape, B::Escape),
        (I::LeftCtrl, B::ControlLeft),
        (I::LeftShift, B::ShiftLeft),
        (I::LeftAlt, B::AltLeft),
        (I::LeftSuper, B::SuperLeft),
        (I::RightCtrl, B::ControlRight),
        (I::RightShift, B::ShiftRight),
        (I::RightAlt, B::AltRight),
        (I::RightSuper, B::SuperRight),
        (I::Menu, B::ContextMenu),
        (I::Key0, B::Digit0),
        (I::Key1, B::Digit1),
        (I::Key2, B::Digit2),
        (I::Key3, B::Digit3),
        (I::Key4, B::Digit4),
        (I::Key5, B::Digit5),
        (I::Key6, B::Digit6),
        (I::Key7, B::Digit7),
        (I::Key8, B::Digit8),
        (I::Key9, B::Digit9),
        (I::A, B::KeyA),
        (I::B, B::KeyB),
        (I::C, B::KeyC),
        (I::D, B::KeyD),
        (I::E, B::KeyE),
        (I::F, B::KeyF),
        (I::G, B::KeyG),
        (I::H, B::KeyH),
        (I::I, B::KeyI),
        (I::J, B::KeyJ),
        (I::K, B::KeyK),
        (I::L, B::KeyL),
        (I::M, B::KeyM),
        (I::N, B::KeyN),
        (I::O, B::KeyO),
        (I::P, B::KeyP),
        (I::Q, B::KeyQ),
        (I::R, B::KeyR),
        (I::S, B::KeyS),
        (I::T, B::KeyT),
        (I::U, B::KeyU),
        (I::V, B::KeyV),
        (I::W, B::KeyW),
        (I::X, B::KeyX),
        (I::Y, B::KeyY),
        (I::Z, B::KeyZ),
        (I::F1, B::F1),
        (I::F2, B::F2),
        (I::F3, B::F3),
        (I::F4, B::F4),
        (I::F5, B::F5),
        (I::F6, B::F6),
        (I::F7, B::F7),
        (I::F8, B::F8),
        (I::F9, B::F9),
        (I::F10, B::F10),
        (I::F11, B::F11),
        (I::F12, B::F12),
        (I::Apostrophe, B::Quote),
        (I::Comma, B::Comma),
        (I::Minus, B::Minus),
        (I::Period, B::Period),
        (I::Slash, B::Slash),
        (I::Semicolon, B::Semicolon),
        (I::Equal, B::Equal),
        (I::LeftBracket, B::BracketLeft),
        (I::Backslash, B::Backslash),
        (I::RightBracket, B::BracketRight),
        (I::GraveAccent, B::Backquote),
        (I::CapsLock, B::CapsLock),
        (I::ScrollLock, B::ScrollLock),
        (I::NumLock, B::NumLock),
        (I::PrintScreen, B::PrintScreen),
        (I::Pause, B::Pause),
        (I::Keypad0, B::Numpad0),
        (I::Keypad1, B::Numpad1),
        (I::Keypad2, B::Numpad2),
        (I::Keypad3, B::Numpad3),
        (I::Keypad4, B::Numpad4),
        (I::Keypad5, B::Numpad5),
        (I::Keypad6, B::Numpad6),
        (I::Keypad7, B::Numpad7),
        (I::Keypad8, B::Numpad8),
        (I::Keypad9, B::Numpad9),
        (I::KeypadDecimal, B::NumpadDecimal),
        (I::KeypadDivide, B::NumpadDivide),
        (I::KeypadMultiply, B::NumpadMultiply),
        (I::KeypadSubtract, B::NumpadSubtract),
        (I::KeypadAdd, B::NumpadAdd),
        (I::KeypadEnter, B::NumpadEnter),
        (I::KeypadEqual, B::NumpadEqual),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dear_imgui_new_frame_system(
    mut context: NonSendMut<DearImguiContext>,
    primary_window: Query<(Entity, &Window), With<PrimaryWindow>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    mut received_chars: MessageReader<KeyboardInput>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
) {
    let map = key_map();
    let context = context.as_mut();

    if context.input_state.keys.len() != map.len() {
        context.input_state.keys = vec![false; map.len()];
    }

    let ui_ptr: NonNull<dear_imgui_rs::Ui>;
    {
        let ctx = context.ctx.get_mut().unwrap();

        // Apply managed texture feedback from the render thread before starting a new frame.
        let feedback: Vec<TextureFeedback> = {
            let mut queue = context
                .texture_feedback
                .lock()
                .expect("Failed to lock Dear ImGui texture feedback queue");
            std::mem::take(queue.as_mut())
        };
        if !feedback.is_empty() {
            let _applied = ctx.platform_io_mut().apply_texture_feedback(&feedback);
            for fb in &feedback {
                if fb.status == TextureStatus::Destroyed {
                    context.managed_textures.remove(&fb.id);
                }
            }
        }

        let primary_window_data = primary_window.single().ok().map(|(_, primary)| {
            (
                primary.width(),
                primary.height(),
                primary.scale_factor(),
                primary.cursor_position(),
            )
        });

        if let Some((_, _, scale, _)) = primary_window_data {
            if scale != context.last_display_scale {
                let settings = context.plugin_settings.clone();
                update_display_scale(context.last_display_scale, scale, &settings, ctx);
                context.last_display_scale = scale;
            }
        }

        {
            let io = ctx.io_mut();

            if let Some((width, height, scale, cursor_pos)) = primary_window_data {
                io.set_display_size([width, height]);
                io.set_display_framebuffer_scale([scale, scale]);

                if let Some(pos) = cursor_pos {
                    io.add_mouse_pos_event([pos.x, pos.y]);
                }
            }

            // Keys (edge-triggered).
            for (i, (imgui_key, bevy_key)) in map.iter().enumerate() {
                let down = keyboard.pressed(*bevy_key);
                if context.input_state.keys[i] != down {
                    io.add_key_event(*imgui_key, down);
                    context.input_state.keys[i] = down;
                }
            }

            // Modifier keys (ImGui expects Mod* keys as well).
            let new_ctrl =
                keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
            let new_shift =
                keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
            let new_alt = keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight);
            let new_super =
                keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight);

            if context.input_state.mod_ctrl != new_ctrl {
                io.add_key_event(dear_imgui_rs::Key::ModCtrl, new_ctrl);
                context.input_state.mod_ctrl = new_ctrl;
            }
            if context.input_state.mod_shift != new_shift {
                io.add_key_event(dear_imgui_rs::Key::ModShift, new_shift);
                context.input_state.mod_shift = new_shift;
            }
            if context.input_state.mod_alt != new_alt {
                io.add_key_event(dear_imgui_rs::Key::ModAlt, new_alt);
                context.input_state.mod_alt = new_alt;
            }
            if context.input_state.mod_super != new_super {
                io.add_key_event(dear_imgui_rs::Key::ModSuper, new_super);
                context.input_state.mod_super = new_super;
            }

            // Mouse buttons (edge-triggered).
            let left = mouse.pressed(bevy::input::mouse::MouseButton::Left);
            if context.input_state.mouse_left != left {
                io.add_mouse_button_event(dear_imgui_rs::input::MouseButton::Left, left);
                context.input_state.mouse_left = left;
            }
            let right = mouse.pressed(bevy::input::mouse::MouseButton::Right);
            if context.input_state.mouse_right != right {
                io.add_mouse_button_event(dear_imgui_rs::input::MouseButton::Right, right);
                context.input_state.mouse_right = right;
            }
            let middle = mouse.pressed(bevy::input::mouse::MouseButton::Middle);
            if context.input_state.mouse_middle != middle {
                io.add_mouse_button_event(dear_imgui_rs::input::MouseButton::Middle, middle);
                context.input_state.mouse_middle = middle;
            }

            // Text input.
            for event in received_chars.read() {
                if event.state != ButtonState::Pressed {
                    continue;
                }

                match &event.logical_key {
                    Key::Character(c) => {
                        if let Some(last) = c.chars().last() {
                            io.add_input_character(last);
                        }
                    }
                    Key::Dead(Some(c)) => {
                        io.add_input_character(*c);
                    }
                    Key::Space => {
                        io.add_input_character(' ');
                    }
                    _ => {}
                }
            }

            for e in mouse_wheel.read() {
                io.add_mouse_wheel_event([e.x, e.y]);
            }
        }

        let ui_ref = ctx.frame();
        ui_ptr = NonNull::new(ui_ref as *mut dear_imgui_rs::Ui).unwrap();
    }

    context.ui = Some(ui_ptr);
}
