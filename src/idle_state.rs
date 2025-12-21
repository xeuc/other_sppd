use bevy::prelude::*;


pub fn idle_state_system(
    mut states: crate::components::States, 
    mut shapes: Query<(
        Entity,
        &mut Transform,
        &crate::components::Team,
        &crate::components::AttackRange,
        &crate::components::DetectionRange,
        // &crate::components::Idle,
        Option<&crate::components::Idle>,
    ), With<crate::components::Shape>>,
    time: Res<Time>,
) {
    let all_shapes: Vec<(Entity, Transform, crate::components::Team)> = shapes.iter().map(|(e, tr, te, _, _, _)| (e, *tr, *te)).collect();

    for (entity, mut transform, team, atk_range, dtk_range, idle_0) in shapes.iter_mut() {

        // i need walking step here
        let Some(idle) = idle_0 else {
            continue;
        };

        // For later, this is defined early in the scope it need to be on
        let mut closest_enemy_and_distance_squared: Option<(Entity, f32, f32)> = None; // (entity, distance_squared, rayon_other)

        // I neel ALL entities, not just IDLE ones
        // Let's determine closest enemy
        for (other_entity, other_transform, other_team) in all_shapes.iter() {
            // not itself
            if entity == *other_entity {
                continue;
            }

            // Check collision - distance squared to avoid perf cost of sqrt
            let distance_squared = transform.translation.distance_squared(other_transform.translation);
            // if distance_squared <= (transform.scale.x/2.0 + other_transform.scale.x/2.0).powi(2) {
                
            //     // Collide
            //     // states.entity(entity).transition(idle, Colliding {});
            //     // State is EITHER idle, colliding.. it's mutually exlusive, 
            //     // If collide, I don't give a freack about other stuff: FIX THE COLLIDE, all that matter rn
            //     // continue;
            // to remove
            //     commands.entity(entity).insert(crate::components::CollidingComponent);
            // } else {
            //     // Todo refacto, the current entity should path find his way to 
            // to remove
            //     commands.entity(entity).remove::<crate::components::CollidingComponent>();
            // }

            // Filter same team => ranges have a sens only about others team shapes...........
            if team == other_team {
                continue;
            }

            // get the closest one
            // keep closest one
            if closest_enemy_and_distance_squared.is_none() || distance_squared < closest_enemy_and_distance_squared.unwrap().1 {
                closest_enemy_and_distance_squared = Some((*other_entity, distance_squared, other_transform.scale.x/2.0));
            }
        }

        let dir = Vec3::new(idle.0, 0.0, 0.0);

        // Do the math on the closest entity only
        if let Some((closest_target, distance_squared, other_rayon)) = closest_enemy_and_distance_squared {

            match distance_squared {
                // ranges take in account self size, but not the other's shape size oc
                d if d > (dtk_range.0 + other_rayon).powi(2) => { /* still idle, retry in 2 sec */ transform.translation += dir * 1.0 * time.delta_secs(); },
                d if d > (atk_range.0 + other_rayon).powi(2) => { /* detecting */ states.entity(entity).transition(idle, crate::components::Walking(closest_target)); },
                d if d > (transform.scale.x/2.0 + other_rayon).powi(2) => { /* attacking */ states.entity(entity).transition(idle, crate::components::Attacking(closest_target)); },
                _ => { /* colliding, should not reach that */ states.entity(entity).transition(idle, crate::components::Colliding {}); },
            }
        } else {

            transform.translation += dir * 1.0 * time.delta_secs();
        }
        

    }

}
