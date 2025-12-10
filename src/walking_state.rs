use bevy::prelude::*;

pub fn move_towards_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &crate::components::AttackRange, &crate::components::MotionState)>,
    time: Res<Time>,
) {
    // Snapshot readonly
    let all: Vec<(Entity, Transform, crate::components::MotionState)> =
        query.iter().map(|(e, t, _a, s)| (e, *t, *s)).collect();

    for (entity, mut transform, atk_range, state) in query.iter_mut() {
        
        if let crate::components::MotionState::Walking(target) = state {
            let Some((_eee, target_transform, _a)) = all.iter().find(|(e, _, _)| *e == *target) else {
                continue;
            };

            let dir = target_transform.translation - transform.translation;
            let d = dir.length();
            let rayon_of_the_other_ball = target_transform.scale.x / 2.0;

            if d <= atk_range.0 - rayon_of_the_other_ball {
                commands.entity(entity)
                    .insert(crate::components::MotionState::Attacking(*target));
            } else {
                let speed = 1.0;
                transform.translation += dir.normalize() * speed * time.delta_secs();
            }
        }
    }
}


pub fn move_right(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<crate::components::MoveRight>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in query.iter_mut() {
        let speed = 2.0;
        transform.translation -= Vec3::X.normalize() * speed * time.delta_secs();
        commands.entity(entity).remove::<crate::components::MoveRight>();
    }
}

pub fn move_left(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<crate::components::MoveLeft>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in query.iter_mut() {
        let speed = 2.0;
        transform.translation += Vec3::X.normalize() * speed * time.delta_secs();
        commands.entity(entity).remove::<crate::components::MoveLeft>();
    }
}