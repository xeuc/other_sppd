
use bevy::{color::palettes::tailwind::CYAN_300, prelude::*};


pub fn resolve_material_state(
    mut q: Query<(
        &mut MeshMaterial3d<StandardMaterial>,
        &crate::components::OriginalMaterial,
        Option<&crate::components::BeingHovered>,
        Option<&crate::components::CollidingComponent>,
        &crate::components::Team,
    )>,
    // team_colors: Res<crate::components::TeamColors>,
    // gray: Res<Handle<StandardMaterial>>,
    // hover_color: Res<Handle<StandardMaterial>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    // to put in ressource handler :)
    let hover_matl = mats.add(Color::from(CYAN_300));
    let _red = mats.add(Color::srgb(0.8, 0.1, 0.2));
    let _blue = mats.add(Color::srgb(0.1, 0.2, 0.8));
    let collision_color = mats.add(Color::srgb(0.2, 0.2, 0.2));

    for (mut mat, original, hovered, collided, _team) in q.iter_mut() {
        
        if hovered.is_some() {
            mat.0 = hover_matl.clone();
        }
        else if collided.is_some() {
            mat.0 = collision_color.clone();
        }
        else {
            mat.0 = original.0.clone();
        }
    }
}


