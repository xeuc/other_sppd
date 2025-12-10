

use bevy::{color::palettes::tailwind::*, picking::pointer::PointerInteraction, prelude::*};

mod spawn_ball;
mod components;
mod idle_state;
mod draw_guizmos;
mod walking_state;
mod change_color;
mod attack_state;

fn main() {
    App::new()
        // MeshPickingPlugin is not a default plugin
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        // .add_message::<crate::components::DeathEvent>()

        .add_systems(Startup, setup_scene)
        .add_systems(Update, draw_mesh_intersections)
        .add_systems(Update, crate::walking_state::move_towards_system)
        // .add_systems(Update, crate::walking_state::apply_move_direction)
        .add_systems(Update, crate::attack_state::attack_system)
        .add_systems(Update, crate::attack_state::kill_system)
        .add_systems(Update, crate::walking_state::move_right)
        .add_systems(Update, crate::walking_state::move_left)
        // .add_systems(Update, detect_system)
        // .add_systems(Update, detect_collisions)
        // .add_systems(Update, apply_collision_material)
        // .add_systems(Update, crate::walking_state::detection_and_collision_system)
        .add_systems(Update, crate::idle_state::collision_system)
        .add_systems(Update, crate::draw_guizmos::draw_gizmos_system)
        .add_systems(Update, crate::change_color::resolve_material_state)

        .run();
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
    ));

    // Instructions
    commands.spawn((
        Text::new("Hover over the shapes to pick them\nDrag to rotate"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}



/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}




/// An observer to rotate an entity when it is dragged
// fn rotate_on_drag(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
//     let mut transform = transforms.get_mut(drag.entity).unwrap();
//     transform.rotate_y(drag.delta.x * 0.02);
//     transform.rotate_x(drag.delta.y * 0.02);
// }

// fn collision_material_system(
//     mut mats_query: Query<&mut MeshMaterial3d<StandardMaterial>>,
//     pos_query: Query<(Entity, &Transform, &crate::components::OriginalMaterial)>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
// ) {
//     // Récupère la liste d'entités (on garde l'ordre pour itérer ensuite)
//     let entities: Vec<Entity> = pos_query.iter().map(|(e, _, _)| e).collect();

//     // Set des entités qui sont en collision avec quelqu'un
//     let mut collided: HashSet<Entity> = HashSet::new();

//     // Crée un matériau collision une seule fois
//     let collision_mat: Handle<StandardMaterial> = materials.add(StandardMaterial {
//         base_color: Color::srgb(0.2, 0.2, 0.2),
//         ..Default::default()
//     });

//     // double loop 🤮
//     for i in 0..entities.len() {
//         for j in (i + 1)..entities.len() {
//             let e1 = entities[i];
//             let e2 = entities[j];

//             // Lecture des composants (gestion d'erreur minimale ici)
//             let (_, t1, _) = match pos_query.get(e1) {
//                 Ok(v) => v,
//                 Err(_) => continue,
//             };
//             let (_, t2, _) = match pos_query.get(e2) {
//                 Ok(v) => v,
//                 Err(_) => continue,
//             };

//             let dx = t2.translation.x - t1.translation.x;
//             let dz = t2.translation.z - t1.translation.z; // si tu travailles en XZ
//             let dist2 = dx * dx + dz * dz;
//             let radius = t1.scale.x/2.0 + t2.scale.x/2.0;

//             // Do collide?
//             if dist2 < radius * radius {
//                 collided.insert(e1);
//                 collided.insert(e2);
//             }
//             // Do attack?
//             if dist2 < radius * radius {
//                 collided.insert(e1);
//                 collided.insert(e2);
//             }
//             // Do detect?
//         }
//     }

//     // Applique les matériaux selon le drapeau collided
//     for e in entities {
//         if collided.contains(&e) {
//             if let Ok(mut m) = mats_query.get_mut(e) {
//                 m.0 = collision_mat.clone();
//             }
//         } else {
//             // restaure le matériau d'origine depuis le composant OriginalMaterial
//             if let Ok((_, _, om)) = pos_query.get(e) {
//                 if let Ok(mut m) = mats_query.get_mut(e) {
//                     m.0 = om.0.clone();
//                 }
//             }
//         }
//     }
// }


// fn tatatatatatatataatata(
//     entity: &Entity,
// ) {
//     let dist = 3.0;
//     if dist < entity.size {
//         return "collision";
//     }
//     if dist < entity.attack_range {
//         return "attack_range";
//     }
//     if dist < entity.detection_range {
//         return "detection_range";
//     }
//     return "none";
// }


// To put in paralel later
// fn detect_system(
//     mut gizmos: Gizmos,
//     q: Query<(Entity, &DetectionRange, &AttackRange, &Transform, &Team)>,
// ) {
//     // Crée un matériau collision une seule fois
//     let collision_mat: Handle<StandardMaterial> = materials.add(StandardMaterial {
//         base_color: Color::srgb(0.2, 0.2, 0.2),
//         ..Default::default()
//     });
//     for (entity, detection_range, attack_range, transform, team) in q.iter() {
//         // Draw guizmos
//         let ground_up = Vec3::Y; // Assuming ground normal is Y axis
//         let isometry = Isometry3d::new(
//                 transform.translation + ground_up * 0.01,
//                 Quat::from_rotation_arc(Vec3::Z, ground_up),
//             );
//         gizmos.circle(isometry, transform.scale.x/2.0, Color::srgb(0.9, 0.0, 0.1));
//         gizmos.circle(isometry, attack_range.0/2.0, Color::srgb(0.7, 0.7, 0.1));
//         gizmos.circle(isometry, detection_range.0/2.0, Color::srgb(0.0, 0.8, 0.3));
//         // end draw guizmos

//         let mut closest: Option<(Entity, f32)> = None;
//         for (other, _other_detection_range, _other_attack_range, other_transform, other_team) in q.iter() {
//             // Avoid itself
//             if entity == other { continue; }
//             // Avoid same team
//             if team == other_team { continue; }
            
//             let distance_squared = transform.translation.distance_squared(other_transform.translation);
//             closest = closer(closest, (other, distance_squared));
//         }

//         if let Some((enemy, distance_squared)) = closest {
//             let dx = t2.translation.x - t1.translation.x;
//             let dz = t2.translation.z - t1.translation.z;
//             let dist2 = dx * dx + dz * dz;
//             let radius = c1.half_size + c2.half_size;

//             if dist2 < radius * radius {
//                 collided.insert(e1);
//                 collided.insert(e2);
//             }
//             if distance_squared <=  (c1.half_size + c2.half_size).powi(2) {
//                 commands.entity(entity).material = collision_mat.clone();
//             }
//             // Attack if within attack range
//             if distance_squared < attack_range.0 {
//                 // println!("{:?} attacks {:?}", entity, enemy);
//                 // Wait 5 sec 
//                 // std::thread::sleep(std::time::Duration::from_secs(1));
//             }
//             // Detect if within detection range
//             if distance_squared < detection_range.0 {
//                 // println!("{:?} detects {:?}", entity, enemy);
//             }
//         }
//     }
// }


fn _closer(a: Option<(Entity, f32)>, b: (Entity, f32)) -> Option<(Entity, f32)> {
    match a {
        None => Some(b),
        Some((_, da)) if b.1 < da => Some(b),
        _ => a,
    }
}



// #[derive(Resource, Default)]
// pub struct Collided(pub HashSet<Entity>);

// fn detect_collisions(
//     mut collided: ResMut<Collided>,
//     q: Query<(Entity, &Transform, &Collider, &Team)>,
// ) {
//     collided.0.clear();

//     let entities: Vec<_> = q.iter().collect();

//     for i in 0..entities.len() {
//         let (e1, t1, c1, team1) = entities[i];
//         for j in i + 1..entities.len() {
//             let (e2, t2, c2, team2) = entities[j];

//             if team1 == team2 { continue; }

//             let dx = t2.translation.x - t1.translation.x;
//             let dz = t2.translation.z - t1.translation.z;
//             let dist2 = dx * dx + dz * dz;
//             let radius = c1.half_size + c2.half_size;

//             if dist2 < radius * radius {
//                 collided.0.insert(e1);
//                 collided.0.insert(e2);
//             }
//         }
//     }
// }


// fn apply_collision_material(
//     collided: Res<Collided>,
//     mut q: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>, &OriginalMaterial)>,
//     mut mats: ResMut<Assets<StandardMaterial>>,
// ) {
//     let collision_color = Color::srgb(0.2, 0.2, 0.2);

//     for (entity, mut mat_handle, _original) in q.iter_mut() {
//         if collided.0.contains(&entity) {
//             // gris = collision
//             let mat = mats.get_mut(&mat_handle.0).unwrap();
//             mat.base_color = collision_color;
//         } else {
//             // TODO : si tu veux revenir à la couleur d’origine
//         }
//     }
// }

