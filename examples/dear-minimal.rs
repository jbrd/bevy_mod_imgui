#[cfg(feature = "dear-imgui-rs")]
use bevy::prelude::*;

#[cfg(feature = "dear-imgui-rs")]
use bevy_mod_imgui::prelude::*;

#[cfg(feature = "dear-imgui-rs")]
#[derive(Resource)]
struct ImguiState {
    demo_window_open: bool,
}

#[cfg(not(feature = "dear-imgui-rs"))]
fn main() {
    eprintln!("This example requires `--no-default-features --features dear-imgui-rs`.");
}

#[cfg(feature = "dear-imgui-rs")]
fn main() {
    App::new()
        .insert_resource(ClearColor(bevy::prelude::Color::srgb(0.2, 0.2, 0.2)))
        .insert_resource(ImguiState {
            demo_window_open: true,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(DearImguiPlugin::default())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera3d::default());
        })
        .add_systems(Update, imgui_example_ui)
        .run();
}

#[cfg(feature = "dear-imgui-rs")]
fn imgui_example_ui(mut context: NonSendMut<DearImguiContext>, mut state: ResMut<ImguiState>) {
    let ui = context.ui();
    if state.demo_window_open {
        ui.show_demo_window(&mut state.demo_window_open);
    }
}
