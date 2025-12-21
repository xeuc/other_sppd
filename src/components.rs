
use bevy::prelude::*;

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
pub struct Shape;


#[derive(Component, Clone, Copy)]
pub struct AttackRange(pub f32);

#[derive(Component)]
pub struct Life {
    pub hp: f64,
    pub maxhp: f64,
}

#[derive(Component)]
pub struct Attack {
    pub degat: f64,
}

#[derive(Component, Clone, Copy)]
pub struct DetectionRange(pub f32);


// User put cursor on ball
#[derive(Component)]
pub struct BeingHovered;



#[derive(Component)]
pub struct OriginalMaterial(pub Handle<StandardMaterial>);


// to remove
#[derive(Component)]
pub struct CollidingComponent;


#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum Team {
    Red,
    Blue,
}


#[derive(Component)]
pub struct AttackCooldown {
    pub timer: Timer,
}


// #[derive(Component)]
// pub struct MoveRight;

// #[derive(Component)]
// pub struct MoveLeft;




// #### IMPORTS ###############################################
use bevy::ecs::system::{EntityCommands, SystemParam};


// #### GENERALS ###############################################
pub trait State<T>: Component {}

pub struct NpcState {}

// System parameter for managing states
#[derive(SystemParam)]
pub struct States<'w, 's> {
    commands: Commands<'w, 's>,
}

impl<'w, 's> States<'w, 's> {
    pub fn entity(&mut self, entity: Entity) -> EntityStates<'_> {
        EntityStates {
            commands: self.commands.entity(entity),
        }
    }
}

// Wrapper for managing state transitions on an entity
pub struct EntityStates<'a> {
    commands: EntityCommands<'a>,
}

impl<'a> EntityStates<'a> {
    pub fn transition<S, F, T>(&mut self, _from: &F, to: T) where F: State<S>, T: State<S> {
        self.commands.remove::<F>();
        self.commands.insert(to);
    }
}










// #### CUSTOM STATES ###############################################

#[derive(Component)]
pub struct Idle(pub f32);
impl State<NpcState> for Idle {}

#[derive(Component)]
pub struct Walking(pub Entity);
impl State<NpcState> for Walking {

}

#[derive(Component)]
pub struct Attacking(pub Entity);
impl State<NpcState> for Attacking {}

#[derive(Component)]
pub struct Colliding {}
impl State<NpcState> for Colliding {}








// Example system using States
// fn some_system(
//     mut states: States, 
//     query: Query<(Entity, &Idle)>
// ) {
//     for (entity, idle) in query.iter() {
//         states.entity(entity).transition(idle, Idle {});
//     }
// }





