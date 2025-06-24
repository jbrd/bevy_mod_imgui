use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;

#[derive(Resource)]
struct ImguiState;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgba(0.2, 0.2, 0.2, 1.0)))
        .insert_resource(ImguiState {})
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera3d::default());
        })
        .add_systems(Update, imgui_example_ui);
    app.run();
}

fn imgui_example_ui(_context: NonSendMut<ImguiContext>, _state: ResMut<ImguiState>) {
    // Example to regression test our workaround for https://github.com/Yatekii/imgui-wgpu-rs/issues/114
}
