
use bevy::prelude::*;
use bevy::color::palettes::tailwind::*;
use rand::prelude::*;


pub fn spawn_ball(
    event: On<Pointer<Release>>,
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>
) {
    let pos = event.hit.position.unwrap();
    let right_click = event.button == PointerButton::Primary;
    let hover_matl = mats.add(Color::from(CYAN_300));
    let pressed_matl = mats.add(Color::from(YELLOW_300));
    let red = mats.add(Color::srgb(0.8, 0.1, 0.2));
    let blue = mats.add(Color::srgb(0.1, 0.2, 0.8));
    let original_material = match right_click {
        true => blue.clone(),
        false => red.clone(),
    };
    let team = match right_click {
        true => crate::components::Team::Red,
        false => crate::components::Team::Blue,
    };
    let mut rng = rand::rng();
    let size = rng.random_range(1.0..2.0);
    let atk_range = rng.random_range(2.0..5.0);
    let dtk_range = rng.random_range(5.0..15.0);
    let life = rng.random_range(10.0..15.0);
    let atk = rng.random_range(1.0..2.0);
    let atk_cooldown = rng.random_range(0.8..1.2);
    let _dir = if right_click { 1.0 } else { -1.0 };
    let _entity = cmds.spawn((
        crate::components::Shape,
        Mesh3d(meshes.add(Sphere::default().mesh().ico(5).unwrap())),
        MeshMaterial3d(original_material.clone()),
        Transform::from_translation(pos + Vec3::Y).with_scale(Vec3::splat(size)),
        crate::components::OriginalMaterial(original_material.clone()),
        crate::components::DetectionRange(dtk_range),
        crate::components::AttackRange(atk_range),
        crate::components::Life{ hp: life, maxhp: life },
        crate::components::Attack{ degat: atk },
        team,
        crate::components::AttackCooldown {
            timer: Timer::from_seconds(atk_cooldown, TimerMode::Repeating),
        },
        crate::components::MotionState::Idle,
    ))
        .observe(mark_hover::<Pointer<Over>>())
        .observe(unmark_hover::<Pointer<Out>>())
        // .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
        // .observe(update_material_on::<Pointer<Out>>(original_material.clone()))
        .observe(update_material_on::<Pointer<Press>>(pressed_matl.clone()))
        .observe(update_material_on::<Pointer<Release>>(hover_matl.clone()))
        // .observe(rotate_on_drag)
        .id();
}

// TO REMOVE LATER
/// Returns an observer that updates the entity's material to the one specified.
fn update_material_on<E: EntityEvent>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(On<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    // An observer closure that captures `new_material`. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // material. Instead, the event type is a generic, and the material is passed in.
    move |event, mut query| {
        if let Ok(mut material) = query.get_mut(event.event_target()) {
            material.0 = new_material.clone();
        }
    }
}

fn mark_hover<E: EntityEvent>() -> impl Fn(On<E>, Commands) {
    move |event: On<E>, mut cmds: Commands| {
        cmds.entity(event.event_target()).insert(crate::components::BeingHovered);
    }
}



fn unmark_hover<E: EntityEvent>() -> impl Fn(On<E>, Commands) {
    move |event: On<E>, mut cmds: Commands| {
        cmds.entity(event.event_target()).remove::<crate::components::BeingHovered>();
    }
}

