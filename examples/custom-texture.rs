use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;

#[derive(Default, Resource)]
struct ImguiState {
    demo_window_open: bool,
    texture_handle: Option<Handle<Image>>,
    texture_id: usize,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgba(0.2, 0.2, 0.2, 1.0)))
        .insert_resource(ImguiState {
            demo_window_open: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera3d::default());
        })
        .add_systems(Update, imgui_example_ui)
        .add_systems(Startup, startup);
    app.run();
}

fn startup(mut state: ResMut<ImguiState>, asset_server: Res<AssetServer>) {
    load_texture(&mut state, &asset_server);
}

fn load_texture(state: &mut ResMut<ImguiState>, asset_server: &Res<AssetServer>) {
    state.texture_handle = Some(asset_server.load("Textures/example_texture.png"));
}

fn register_texture_if_loaded(
    state: &mut ResMut<ImguiState>,
    asset_server: &Res<AssetServer>,
    context: &mut NonSendMut<ImguiContext>,
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

fn unload_texture(state: &mut ResMut<ImguiState>, context: &mut NonSendMut<ImguiContext>) {
    context.unregister_bevy_texture(&TextureId::new(state.texture_id));
    state.texture_handle = Default::default();
    state.texture_id = 0;
}

fn imgui_example_ui(
    mut state: ResMut<ImguiState>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut context: NonSendMut<ImguiContext>,
) {
    register_texture_if_loaded(&mut state, &asset_server, &mut context);

    // Do we have a texture?
    let has_texture = state.texture_id != 0;
    let mut should_unload_texture = false;
    let mut should_load_texture = false;

    let ui = context.ui();
    if state.demo_window_open {
        let window = ui.window("Custom Texture");
        window
            .size([700.0, 700.0], imgui::Condition::FirstUseEver)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("This is a custom Bevy texture");
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
                        let image_control =
                            imgui::Image::new(TextureId::new(state.texture_id), image_size);
                        image_control.build(ui);
                    }
                } else if ui.button("Load Texture") {
                    should_load_texture = true;
                }
            });
    }

    if should_load_texture {
        load_texture(&mut state, &asset_server);
    } else if should_unload_texture {
        unload_texture(&mut state, &mut context);
    }
}
