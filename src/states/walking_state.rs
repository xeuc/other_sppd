use bevy::prelude::*;

// PS: there no notion of "team" here, the walking state make a ball blindly goes toward another one
// pub fn walking_state_system(
//     mut states: crate::components::States, 
//     mut query: Query<(
//         Entity,
//         &mut Transform,
//         &crate::components::AttackRange,
//         &crate::components::Walking,
//     ), With<crate::components::Shape>>,
//     time: Res<Time>,
//     all_shapes: Query<(
//         Entity,
//         &Transform,
//         &crate::components::Team,
//     ), With<crate::components::Shape>>,
// ) {
//     for (entity, mut transform, atk_range, target) in query.iter_mut() {
        
//         // Need all shaped to grab position of entity inside Walking state ( Walking(THIS_ONE) ) of self entity
//         let Some((_eee, other_transform, _a)) = all_shapes.iter().find(|(e, _, _)| *e == target.0) else {
//             continue;
//         };

//         // all_shapes.get(target.0).is_err();


//         let dir = other_transform.translation - transform.translation;
//         // let distance_squared = transform.translation.distance_squared(other_transform.translation);
//         let distance_squared = dir.length_squared();
//         let other_rayon = other_transform.scale.x/2.0;
//         match distance_squared {
//             // ranges take in account self size, but not the other's shape size oc
//             d if d > f32::powf(atk_range.0 + other_rayon, 2.0) => { /* ACTUALLY GO TOWARD */ transform.translation += dir.normalize() * {let speed = 1.0; speed} * time.delta_secs(); },
//             d if d > f32::powf(transform.scale.x/2.0 + other_rayon, 2.0) => { /* detecting */ states.entity(entity).transition(target, crate::components::Attacking(target.0)); },
//             _ => { /* colliding, should not reach that */ states.entity(entity).transition(target, crate::components::Colliding {}); },
//         }

//     }
// }


// pub fn move_right(
//     mut commands: Commands,
//     mut query: Query<(Entity, &mut Transform), With<crate::components::MoveRight>>,
//     time: Res<Time>,
// ) {
//     for (entity, mut transform) in query.iter_mut() {
//         let speed = 2.0;
//         transform.translation -= Vec3::X.normalize() * speed * time.delta_secs();
//         commands.entity(entity).remove::<crate::components::MoveRight>();
//     }
// }

// pub fn move_left(
//     mut commands: Commands,
//     mut query: Query<(Entity, &mut Transform), With<crate::components::MoveLeft>>,
//     time: Res<Time>,
// ) {
//     for (entity, mut transform) in query.iter_mut() {
//         let speed = 2.0;
//         transform.translation += Vec3::X.normalize() * speed * time.delta_secs();
//         commands.entity(entity).remove::<crate::components::MoveLeft>();
//     }
// }







// PS: there no notion of "team" here, the walking state make a ball blindly goes toward another one
// pub fn walking_state_system(
//     mut states: crate::components::States, 
//     mut query: Query<(
//         Entity,
//         &mut Transform,
//         &crate::components::AttackRange,
//         // &crate::components::Walking,
//     ), With<crate::components::Shape>>,
//     time: Res<Time>,
// ) {
//     // Snapshot readonly (ugly hard fix)
//     let all_shapes: Vec<(Entity, Transform)> =
//         query.iter().map(|(e, t, _)| (e, *t)).collect();
         
//     for (entity, mut transform, atk_range) in query.iter_mut() {
        
//         // if entity's state is "WALKING", then grab his target entity
//         // ???

//         // GET THE POSITION of the ENTITY: target
//         // Need all shaped to grab position of entity inside Walking state ( Walking(THIS_ONE) ) of self entity
//         let Some((_, other_transform)) = all_shapes.iter().find(|(e, _)| *e == target) else {
//             continue;
//         };
//         // all_shapes.get(target.0).is_err();


//         let dir = other_transform.translation - transform.translation;
//         // let distance_squared = transform.translation.distance_squared(other_transform.translation);
//         let distance_squared = dir.length_squared();
//         let other_rayon = other_transform.scale.x/2.0;
//         match distance_squared {
//             // ranges take in account self size, but not the other's shape size oc
//             d if d > f32::powf(atk_range.0 + other_rayon, 2.0) => { /* ACTUALLY GO TOWARD */ transform.translation += dir.normalize() * {let speed = 1.0; speed} * time.delta_secs(); },
//             d if d > f32::powf(transform.scale.x/2.0 + other_rayon, 2.0) => { /* detecting */ states.entity(entity).transition(target, crate::components::Attacking(target.0)); },
//             _ => { /* colliding, should not reach that */ states.entity(entity).transition(target, crate::components::Colliding {}); },
//         }

//     }
// }


pub fn walking_state_system(
    mut states: crate::components::States,
    mut query: Query<(
        Entity,
        &mut Transform,
        &crate::components::AttackRange,
        // cannot use this as my all shape querry will not contain all entities
        // and I don't want the current entities to walk to only entities that are in "walking" state!
        // &crate::components::Walking,
        Option<&crate::components::Walking>,
    ), With<crate::components::Shape>>,
    time: Res<Time>,
) {
    let all_shapes: Vec<(Entity, Transform)> = query.iter().map(|(e, t, _, _)| (e, *t)).collect();

    for (entity, mut transform, atk_range, walking) in query.iter_mut() {

        // i need walking step here
        let Some(walking) = walking else {
            continue;
        };

        // target is..
        let target = walking.0;

        let Some((_, other_transform)) = all_shapes.iter().find(|(e, _)| *e == target) else {
            continue;
        };

        let dir = other_transform.translation - transform.translation;
        let distance_squared = dir.length_squared();
        let other_rayon = other_transform.scale.x / 2.0;

        match distance_squared {
            d if d > (atk_range.0 + other_rayon).powi(2) => { transform.translation += dir.normalize() * 1.0 * time.delta_secs(); }
            d if d > (transform.scale.x / 2.0 + other_rayon).powi(2) => { states.entity(entity).transition(walking, crate::components::Attacking(target)); }
            _ => { states.entity(entity).transition(walking, crate::components::Colliding {}); }
        }
    }
}
