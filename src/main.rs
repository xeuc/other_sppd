

use bevy::{color::palettes::tailwind::*, prelude::*};

mod spawn_ball;
mod components;
mod draw_guizmos;
mod change_color;
mod ui;

mod states;


fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        
        .add_systems(Startup, setup_scene)
        .add_systems(Update, crate::states::attack_state::kill_system)
        .add_systems(Update, crate::draw_guizmos::draw_gizmos_system)
        .add_systems(Update, crate::draw_guizmos::draw_mesh_intersections)
        .add_systems(Update, crate::change_color::resolve_material_state)

        // states related systems
        .add_systems(Update, crate::states::idle_state::idle_state_system)
        .add_systems(Update, crate::states::walking_state::walking_state_system)
        .add_systems(Update, crate::states::attack_state::attack_state_system)

        // ui
        .add_systems(Startup, crate::ui::setup_ui)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)

        .run();
}




use bevy::{
    color::palettes::css::{GOLD, ORANGE},
    ui::widget::NodeImageMode,
};


fn button_system(
    mut interaction_query: Query<
        (&Interaction, &Children, &mut ImageNode),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, children, mut image) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                **text = "Press".to_string();
                image.color = GOLD.into();
            }
            Interaction::Hovered => {
                **text = "Hover".to_string();
                image.color = ORANGE.into();
            }
            Interaction::None => {
                **text = "Button".to_string();
                image.color = Color::WHITE;
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let image = asset_server.load("card.png");

    let slicer = TextureSlicer {
        border: BorderRect::all(22.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    };


    commands.spawn((
        Button,
        ImageNode {
            image: image.clone(),
            image_mode: NodeImageMode::Sliced(slicer.clone()),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ))
    .with_child((
        Text::new("Button"),
        TextFont {
            font: asset_server.load("FiraSans-Bold.ttf"),
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
    ));

}









fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Set up the materials.
    let _white_matl = materials.add(Color::WHITE);
    // let ground_matl = materials.add(Color::from(GRAY_300));
    let _hover_matl = materials.add(Color::from(CYAN_300));
    let _pressed_matl = materials.add(Color::from(YELLOW_300));

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        // Pickable::IGNORE, // Disable picking for the ground plane.
    ))
    .observe(|event: On<Pointer<Release>>, cmds: Commands, meshes: ResMut<Assets<Mesh>>, mats: ResMut<Assets<StandardMaterial>>| {
        // call the function from spawn_ball.rs (use snake_case and full path)
        crate::spawn_ball::spawn_ball(event, cmds, meshes, mats);
    })
    ;

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        Camera { order: 0, ..default() },
    ));
}




