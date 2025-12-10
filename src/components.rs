
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



// NO ONE IN ANY ON MY RANGE
// NONE, component does not exist
// -----------------------------------------------------

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum MotionState {
    Idle,
    Walking(Entity),
    Attacking(Entity),
    Colliding,
}

// #[derive(Message)]
// pub struct DeathEvent(pub Entity);

#[derive(Component)]
pub struct MoveRight;

#[derive(Component)]
pub struct MoveLeft;






