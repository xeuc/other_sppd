use bevy::prelude::*;
use bevy::gizmos::gizmos::Gizmos;
use bevy::math::Isometry3d;

pub fn _draw_gizmos_system(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &crate::components::AttackRange, &crate::components::DetectionRange), With<crate::components::Shape>>,
) {
    for (transform, attack_range, detection_range) in &query {
        let ground_up = Vec3::Y;

        let isometry = Isometry3d::new(
            transform.translation + ground_up * 0.01,
            Quat::from_rotation_arc(Vec3::Z, ground_up),
        );

        gizmos.circle(
            isometry,
            transform.scale.x / 2.0,
            Color::srgb(0.9, 0.0, 0.1),
        );

        gizmos.circle(
            isometry,
            attack_range.0,
            Color::srgb(0.7, 0.7, 0.1),
        );

        gizmos.circle(
            isometry,
            detection_range.0,
            Color::srgb(0.0, 0.8, 0.3),
        );
    }
}


pub fn draw_gizmos_system(
    mut gizmos: Gizmos,
    query: Query<(
        &Transform,
        &crate::components::AttackRange,
        &crate::components::DetectionRange,
        &crate::components::Life,
    ), With<crate::components::Shape>>,
) {
    for (transform, attack_range, detection_range, life) in &query {
        let ground_up = Vec3::Y;

        let isometry = Isometry3d::new(
            transform.translation + ground_up * 0.01,
            Quat::from_rotation_arc(Vec3::Z, ground_up),
        );

        gizmos.circle(isometry, transform.scale.x / 2.0, Color::srgb(0.9, 0.0, 0.1));
        gizmos.circle(isometry, attack_range.0, Color::srgb(0.7, 0.7, 0.1));
        gizmos.circle(isometry, detection_range.0, Color::srgb(0.0, 0.8, 0.3));

        // hp bar
        let hp_percent = (life.hp / life.maxhp).clamp(0.0, 1.0) as f32;
        let offset = ground_up * (transform.scale.y * 0.6);
        let width = 0.5 * transform.scale.x;
        let left  = transform.translation + offset + Vec3::new(-width, 0.0, 0.0);
        let right = transform.translation + offset + Vec3::new(width, 0.0, 0.0);

        let mid = transform.translation + offset
            + Vec3::new(-width + (2.0 * width * hp_percent), 0.0, 0.0);

        gizmos.line(mid, right, Color::srgb(0.8, 0.1, 0.1));
        gizmos.line(left, mid, Color::srgb(0.1, 0.9, 0.1));
    }
}
