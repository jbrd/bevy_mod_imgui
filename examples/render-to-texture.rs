//! Shows how to render to a texture. Useful for mirrors, UI, or exporting images.

use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};

use bevy_mod_imgui::prelude::*;

#[derive(Default, Resource)]
struct ImguiState {
    demo_window_open: bool,
    texture_handle: Option<Handle<Image>>,
    texture_id: usize,
}

fn main() {
    App::new()
        .insert_resource(ImguiState {
            demo_window_open: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_mod_imgui::ImguiPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (imgui_example_ui, rotator_system))
        .run();
}

// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ImguiState>,
) {
    // Set up a render target
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill render target with zeroes
    image.resize(size);
    let image_handle = images.add(image);

    // Store texture handle so we can use it during the imgui update
    state.texture_handle = Some(image_handle.clone());

    // Set up a cube that will be rendered to the texture
    let cube_handle = meshes.add(Cuboid::new(4.0, 4.0, 4.0));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The cube that will be rendered to the texture.
    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(cube_material_handle),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        FirstPassCube,
        first_pass_layer.clone(),
    ));

    // Light
    // NOTE: we add the light to both layers so it affects both the rendered-to-texture cube, and the cube on which we display the texture
    // Setting the layer to RenderLayers::layer(0) would cause the main view to be lit, but the rendered-to-texture cube to be unlit.
    // Setting the layer to RenderLayers::layer(1) would cause the rendered-to-texture cube to be lit, but the main view to be unlit.
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        RenderLayers::layer(0).with(1),
    ));

    commands.spawn((
        Camera {
            // render before the "main pass" camera
            order: -1,
            target: image_handle.clone().into(),
            clear_color: Color::WHITE.into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 15.0)).looking_at(Vec3::ZERO, Vec3::Y),
        first_pass_layer,
    ));

    // The main pass camera.
    commands.spawn(Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y));
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in &mut query {
        transform.rotate_x(1.5 * time.delta_secs());
        transform.rotate_z(1.3 * time.delta_secs());
    }
}

fn imgui_example_ui(
    mut state: ResMut<ImguiState>,
    images: Res<Assets<Image>>,
    mut context: NonSendMut<ImguiContext>,
) {
    // Register the texture if we haven't already
    if state.texture_id == 0 {
        if let Some(texture_handle) = &state.texture_handle {
            state.texture_id = context.register_bevy_texture(texture_handle.clone()).id();
        }
    }

    // Do we have a texture?
    let has_texture = state.texture_id != 0;
    let ui = context.ui();
    if state.demo_window_open {
        let window = ui.window("Render To Texture");
        window
            .size([700.0, 700.0], imgui::Condition::FirstUseEver)
            .position([0.0, 0.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("This is a Bevy scene rendered to a texture");
                ui.separator();
                if has_texture {
                    if let Some(image) = images.get(
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
                }
            });
    }
}
