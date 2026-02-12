#[cfg(feature = "dear-imgui-rs")]
use bevy::prelude::*;

#[cfg(feature = "dear-imgui-rs")]
use bevy_mod_imgui::{DearImguiContext, DearImguiPlugin};

#[cfg(feature = "dear-imgui-rs")]
use dear_imgui_rs::texture::TextureId;

#[cfg(feature = "dear-imgui-rs")]
#[derive(Default, Resource)]
struct ImguiState {
    demo_window_open: bool,
    texture_handle: Option<Handle<bevy::prelude::Image>>,
    texture_id: u64,
}

#[cfg(not(feature = "dear-imgui-rs"))]
fn main() {
    eprintln!("This example requires `--no-default-features --features dear-imgui-rs`.");
}

#[cfg(feature = "dear-imgui-rs")]
fn main() {
    App::new()
        .insert_resource(ClearColor(bevy::prelude::Color::srgba(0.2, 0.2, 0.2, 1.0)))
        .insert_resource(ImguiState {
            demo_window_open: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(DearImguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera3d::default());
        })
        .add_systems(Startup, startup)
        .add_systems(Update, imgui_example_ui)
        .run();
}

#[cfg(feature = "dear-imgui-rs")]
fn startup(mut state: ResMut<ImguiState>, asset_server: Res<AssetServer>) {
    load_texture(&mut state, &asset_server);
}

#[cfg(feature = "dear-imgui-rs")]
fn load_texture(state: &mut ResMut<ImguiState>, asset_server: &Res<AssetServer>) {
    state.texture_handle = Some(asset_server.load("Textures/example_texture.png"));
}

#[cfg(feature = "dear-imgui-rs")]
fn register_texture_if_loaded(
    state: &mut ResMut<ImguiState>,
    asset_server: &Res<AssetServer>,
    context: &mut NonSendMut<DearImguiContext>,
) {
    if state.texture_id == 0 {
        if let Some(texture_handle) = &state.texture_handle {
            if asset_server
                .get_load_state(texture_handle.id())
                .unwrap()
                .is_loaded()
            {
                state.texture_id = context.register_bevy_texture(texture_handle.clone()).id();
            }
        }
    }
}

#[cfg(feature = "dear-imgui-rs")]
fn unload_texture(state: &mut ResMut<ImguiState>, context: &mut NonSendMut<DearImguiContext>) {
    context.unregister_bevy_texture(&TextureId::new(state.texture_id));
    state.texture_handle = Default::default();
    state.texture_id = 0;
}

#[cfg(feature = "dear-imgui-rs")]
fn imgui_example_ui(
    mut state: ResMut<ImguiState>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<bevy::prelude::Image>>,
    mut context: NonSendMut<DearImguiContext>,
) {
    register_texture_if_loaded(&mut state, &asset_server, &mut context);

    let has_texture = state.texture_id != 0;
    let mut should_unload_texture = false;
    let mut should_load_texture = false;

    let ui = context.ui();
    if state.demo_window_open {
        ui.window("Custom Texture (dear-imgui-rs)")
            .size([700.0, 700.0], dear_imgui_rs::Condition::FirstUseEver)
            .position([0.0, 0.0], dear_imgui_rs::Condition::FirstUseEver)
            .build(|| {
                ui.text("This is a custom Bevy texture (legacy TextureId path).");
                ui.separator();

                if has_texture {
                    if ui.button("Unload Texture") {
                        should_unload_texture = true;
                    } else if let Some(image) = images.get(
                        state
                            .texture_handle
                            .as_ref()
                            .expect("Should always have a texture at this point"),
                    ) {
                        let image_size = [image.width() as f32, image.height() as f32];
                        ui.image(TextureId::new(state.texture_id), image_size);
                    }
                } else if ui.button("Load Texture") {
                    should_load_texture = true;
                }

                ui.separator();
                ui.checkbox("Show Demo Window", &mut state.demo_window_open);
            });
    }

    if should_load_texture {
        load_texture(&mut state, &asset_server);
    } else if should_unload_texture {
        unload_texture(&mut state, &mut context);
    }
}
