use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;

#[derive(Resource)]
struct ImguiState {
    demo_window_open: bool,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgba(0.2, 0.2, 0.2, 1.0)))
        .insert_resource(ImguiState {
            demo_window_open: true,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera3d::default());
        })
        .add_systems(Update, imgui_example_ui);
    app.run();
}

fn imgui_example_ui(mut context: NonSendMut<ImguiContext>, mut state: ResMut<ImguiState>) {
    let ui = context.ui();
    if state.demo_window_open {
        ui.show_demo_window(&mut state.demo_window_open);
    }
}
