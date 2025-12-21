
use bevy::prelude::*;

// note: like walk state, this state is team blinded
// actually no i need it to know which direction i need to go after defeating an enemy
pub fn attack_state_system(
    mut states: crate::components::States, 
    time: Res<Time>,
    mut attackers: Query<(
        Entity,
        &mut crate::components::AttackCooldown,
        &crate::components::Attack,
        &crate::components::Attacking,
        &crate::components::Team,
    )>,
    mut lives: Query<&mut crate::components::Life>,
) {
    for (_ent, mut cooldown, atk, state, team) in attackers.iter_mut() {
        
        // no target mean enemy defeated! 
        if lives.get_mut(state.0).is_err() {
            match team {
                crate::components::Team::Blue => states.entity(_ent).transition(state, crate::components::Idle(-1.0)),
                crate::components::Team::Red => states.entity(_ent).transition(state, crate::components::Idle(1.0)),
            }
            
            cooldown.timer.reset();
            continue;
        }

        cooldown.timer.tick(time.delta());
        
        if cooldown.timer.is_finished() {
            // the target entity is located in the attack state:
            let target = state.0;
            if let Ok(mut life) = lives.get_mut(target) {
                life.hp -= atk.degat;
            }
        }
    }
}





pub fn kill_system(
    mut commands: Commands,
    // mut events: MessageWriter<crate::components::DeathEvent>,
    query: Query<(Entity, &crate::components::Life)>,
) {
    for (e, life) in &query {
        if life.hp <= 0.0 {
            // events.write(crate::components::DeathEvent(e));
            commands.entity(e).despawn();
        }
    }
}

// pub fn on_death_system(
//     mut events: MessageReader<crate::components::DeathEvent>,
//     mut attackers: Query<(&crate::components::AttackTarget, &mut crate::components::MotionState)>,
// ) {
//     for crate::components::DeathEvent(dead_entity) in events.read() {
//         for (target, mut state) in attackers.iter_mut() {
//             if target.0 == *dead_entity {
//                 // target dead
//                 *state = crate::components::MotionState::Idle;
//             }
//         }
//     }
// }



// pub fn set_attacking(mut query: Query<&mut crate::components::MotionState>) {
//     for mut state in query.iter_mut() {
//         if some_condition {
//             *state = crate::components::MotionState::Attacking;
//         }
//     }
// }

// pub fn on_enter_attack(
//     query: Query<
//         (Entity, &crate::components::MotionState),
//         Changed<crate::components::MotionState>
//     >,
// ) {
//     for (entity, state) in query.iter() {
//         if *state == crate::components::MotionState::Attacking {
//             println!(">>> Entity {entity:?} débute une attaque !");
//         }
//     }
// }

