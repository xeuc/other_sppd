
use bevy::prelude::*;



pub fn setup_ui(
    mut commands: Commands,
) {
    // Instructions
    commands.spawn((
        Text::new("Left  Click to create a Blue ball that goes right
Right Click to create a Red  ball that goes left
Green circle is detection range
Yellow circle is attack range
Red circle is size range (hitbox or hitball lol)
If balls of different teams encounter, they will attack each others"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

