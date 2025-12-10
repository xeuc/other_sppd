use bevy::prelude::*;



// Bug of the size is so big then it collide so nobody move
// and the big cannnot attack because the target is not in range
// so it get rect stuck forever




// pub fn set_attacking(mut query: Query<&mut crate::components::MotionState>) {
//     for mut state in query.iter_mut() {
//             *state = crate::components::MotionState::Attacking;
//     }
// }


// pub fn collision_system_backup(
//     mut commands: Commands,
//     query: Query<(
//         Entity,
//         &Transform,
//         &crate::components::Team,
//         &crate::components::AttackRange,
//         &crate::components::DetectionRange,
//     ), With<crate::components::Shape>>,
// ) {
//     let entities: Vec<(Entity, Transform, crate::components::Team, crate::components::AttackRange, crate::components::DetectionRange)> =
//         query.iter().map(|(e, t, team, a, d)| (e, *t, *team, *a, *d)).collect();
    
//     let mut colliding_entities = Vec::new();

//     // double loop 🤮
//     // the for can"t make a couple of same entity
//     for i in 0..entities.len() {
//         for j in (i + 1)..entities.len() {
//             let (e1, t1, team1, a1, d1) = entities[i];
//             let (e2, t2, team2, a2, d2) = entities[j];

//             if team1 == team2 {
//                 continue; // same team, no interaction
//             }

//             // 2rayon = scale.x 
//             let r1 = t1.scale.x / 2.0;
//             let r2 = t2.scale.x / 2.0;
//             let sum_radius = r1 + r2;
//             let radius_squared = sum_radius * sum_radius;

//             let distance_squared = t1.translation.distance_squared(t2.translation);
//             let distance_squared_to_paroi_not_center = distance_squared - r2 * r2;

//             // do collide?
//             if distance_squared <= radius_squared {
//                 colliding_entities.push(e1);
//                 colliding_entities.push(e2);
//                 commands.entity(e1).remove::<crate::components::MoveDirection>();
//                 commands.entity(e1).remove::<crate::components::GoToward>();
//                 continue;
//             }

//             // do attack range?
//             let radius_atk = a1.0 * a1.0;
//             if distance_squared_to_paroi_not_center <= radius_atk {
//                 // found the closest NEED
//                 // e1.AtkTargetInRange.0 = Some(e2);
//                 commands.entity(e1).insert(crate::components::AttackTarget(e2));
//                 commands.entity(e1).remove::<crate::components::MoveDirection>();
//                 commands.entity(e1).remove::<crate::components::GoToward>();
//                 continue;
//             }

//             // do detection range?
//             let radius_dtk = d1.0 * d1.0;
//             if distance_squared_to_paroi_not_center <= radius_dtk {
//                 // found the closest NEED
//                 // e1.AtkTargetInRange.0 = Some(e2);
//                 commands.entity(e1).insert(crate::components::GoToward(e2));
//                 continue;
//             }

//         }
//     }

//     // global clean
//     for (entity, _, _, _, _) in &entities {
//         commands.entity(*entity).remove::<crate::components::Colliding>();
//     }

//     // tag "colligding" add
//     for e in colliding_entities {
//         commands.entity(e).insert(crate::components::Colliding);
//     }
// }


pub fn collision_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Transform,
        &crate::components::Team,
        &crate::components::AttackRange,
        &crate::components::DetectionRange,
        &mut crate::components::MotionState,
    ), With<crate::components::Shape>>,
) {
    let entities: Vec<(Entity, Transform, crate::components::Team, crate::components::MotionState)> =
        query.iter()
            .map(|(e, t, team, _, _, state)| (e, *t, *team, *state))
            .collect();

    
    for (entity, transform, team, atk_range, detect_range, mut state) in query.iter_mut() {
        commands.entity(entity).remove::<crate::components::MoveLeft>();
        commands.entity(entity).remove::<crate::components::MoveRight>();

        if *state != crate::components::MotionState::Idle {
            continue;
        }

        let mut closest_enemy: Option<(Entity, f32, f32)> = None; // (entity, distance_squared, distance_to_edge)

        for (other_entity, other_transform, other_team, _state) in entities.iter() {
            if *other_entity == entity || *other_team == *team {
                continue;
            }

            // distance center to center
            // not using distance_squared because I add radius calculation AFTER square it 
            // normally doesn't matter for calculation, but code broken so i investigate
            let distance = transform.translation.distance(other_transform.translation);

            // distance to edge of other ball
            let _radius_self = transform.scale.x / 2.0;
            let radius_other = other_transform.scale.x / 2.0;
            let distance_to_edge = distance - radius_other;

            // keep closest one
            if closest_enemy.is_none() || distance_to_edge < closest_enemy.unwrap().2 {
                closest_enemy = Some((*other_entity, distance_to_edge * distance_to_edge, distance_to_edge)); // fuck, need one distance only
            }
        }

        if let Some((target, distance_to_edge_squared, _distance_to_edge_fart)) = closest_enemy {
            let my_size_as_a_ball_aka_rayon_squared = (transform.scale.x / 2.0 + 0.0).powi(2);
            let atk_sq = atk_range.0 * atk_range.0;
            let detect_sq = detect_range.0 * detect_range.0;

            if distance_to_edge_squared <= my_size_as_a_ball_aka_rayon_squared {
                *state = crate::components::MotionState::Colliding;
                commands.entity(entity).insert(crate::components::CollidingComponent);
                // commands.entity(target).insert(crate::components::CollidingComponent);
            } else if distance_to_edge_squared <= atk_sq {
                *state = crate::components::MotionState::Attacking(target);
            } else if distance_to_edge_squared <= detect_sq {
                *state = crate::components::MotionState::Walking(target);
            } else {
                if team == &crate::components::Team::Blue {
                    commands.entity(entity).insert(crate::components::MoveRight);
                } else if team == &crate::components::Team::Red {
                    commands.entity(entity).insert(crate::components::MoveLeft);
                }
                *state = crate::components::MotionState::Idle;
            }
        } else {
            if team == &crate::components::Team::Blue {
                commands.entity(entity).insert(crate::components::MoveRight);
            } else if team == &crate::components::Team::Red {
                commands.entity(entity).insert(crate::components::MoveLeft);
            }
            // no enely close
            *state = crate::components::MotionState::Idle;
        }
    }
}

