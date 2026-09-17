//! Team placement demo (Bevy 0.19.1)
//!
//! Layout: three vertical zones.
//! - Left 20%: black panel, Team Blue's palette (Circle/Tank, Square/Fighter, Triangle/Assassin).
//! - Middle 60%: a real, 60%-width `Camera3d` view of a ground plane.
//! - Right 20%: black panel, Team Red's palette.
//!
//! Architecture: palette/spawner, plus draggable placed units (2D and 3D).
//! The 6 sidebar icons (3 shapes x 2 teams) are permanent UI elements that never move and
//! never change component set -- they are just "spawn buttons". Dragging one spawns a single
//! ephemeral `Ghost` entity that does the moving. Once dropped, that ghost is promoted in
//! place into a permanent unit: a `PlacedUnit` if dropped in the 3D world, or a `Placed2DUnit`
//! if dropped back on a panel.
//!
//! Placed units (2D or 3D) are themselves draggable:
//! - Dragging a placed unit around the 3D world repositions it; overlaps with other placed
//!   units are allowed here (no physics/collision resolution, per spec).
//! - Dragging a placed unit onto a side panel converts it into a `Placed2DUnit` sitting there
//!   ("3D -> 2D, converting it back"); dragging *that* back into the world converts it back,
//!   resolving spawn overlaps exactly like a fresh palette placement (see `resolve_spawn_position`).
//! - Right-clicking mid-drag cancels: a fresh palette-drag just discards its ghost (nothing
//!   existed before), while a placed-unit drag (2D or 3D) is restored to its exact original
//!   position.
//!
//! Every drag (whichever entity started it) is tracked by a single `DragSession` resource, so
//! the drag-move and drag-cancel logic is shared between all three origins (`Palette`,
//! `PlacedUnit`, `Placed2D`); only "how the drag starts" and a few origin-gated branches (the
//! spawn-overlap resolution, and what a cancellation restores) differ.
//!
//! Ghosts, while a drag is active:
//! - A destination ghost/preview follows the cursor: a 2D icon over either side panel, a
//!   translucent 3D mesh over the world (clamped to the dragging team's half, and -- for a new
//!   2D -> 3D placement -- resolved around any spawn overlap so it always shows exactly where
//!   the unit would actually land). It always uses a lighter, translucent version of the
//!   *team's* color.
//! - A static origin marker stays at the drag's starting position for the whole gesture, in a
//!   fixed translucent blue (regardless of team), so the user can always see where the drag
//!   began. It's removed on drop or cancellation.
//!
//! Two cameras, two jobs
//! - `WorldCamera` (Camera3d) is given a real `Camera.viewport` covering exactly the middle
//!   60% of the window (kept in sync every frame), so its projection is centered on that
//!   strip exactly like a camera that only ever knew about that resolution.
//! - A second, plain `Camera2d` (order 1, `ClearColorConfig::None`) renders on top of it, full
//!   window, with no viewport restriction, and is marked `IsDefaultUiCamera` so every UI node
//!   in this app lays out in full, un-clipped window coordinates. That's also why
//!   `Camera::viewport_to_world`/pointer positions can be passed straight through everywhere
//!   below without manually subtracting the world camera's viewport offset -- Bevy's own
//!   conversion functions already account for it.
//!
//! The 2D <-> 3D swap
//! `Node` (UI) and `Mesh3d`/`Transform` (3D) are unrelated component sets, and `Node`
//! positions itself with its own `UiTransform`/`UiGlobalTransform` rather than the regular
//! `Transform`. Combined with the fact that dragged entities are never parented to anything,
//! swapping one component set for the other on `Pointer<Drag>`/`Pointer<DragEnd>` is safe. The
//! palette icons, by contrast, keep their `Node` forever and are ordinary children of their
//! panel -- they never need this trick, since they never change shape.
//!
//! Spawn-overlap resolution: a *new* 2D -> 3D placement (from the palette, or a `Placed2DUnit`
//! being redeployed) must never land on top of an existing 3D unit. `resolve_spawn_position`
//! searches outward in rings from the desired spot for the nearest free one, strictly within
//! the dragging team's half, and is shared by both the ghost preview and the actual drop so
//! they always agree. Repositioning an existing 3D unit (3D -> 3D) never goes through this --
//! that overlap is allowed, per spec.
//!
//! Combat & movement: see the "Combat & movement" section below for the full pipeline. In
//! short, every `PlacedUnit` marches toward the enemy camp by default (`Advancing`), switches
//! to chasing (`Seeking`) or fighting (`Attacking`) the closest detected enemy as those
//! `CombatRanges` change frame to frame, and these three components are mutually exclusive --
//! changing behavior means swapping which one is attached, per the spec's request for
//! state-via-components rather than one global `State` enum.
//!
//! Debug overlay & per-unit variance: every live 3D unit draws its detection/attack
//! `CombatRanges` as flat gizmo circles on the ground, plus a small floating health bar above
//! it, via `draw_unit_gizmos`. Each unit also gets a small random jitter applied to its
//! `Stats` the moment it's first ever placed (see `random_stats`), so two units of the same
//! shape aren't perfectly identical -- that jittered `Stats` then sticks with the entity for
//! its whole life (repositioning or converting it never re-rolls it).

use bevy::{
    camera::Viewport,
    color::palettes::tailwind::*,
    color::Alpha,
    math::Isometry3d,
    picking::pointer::{PointerButton, PointerInteraction},
    prelude::*,
};

fn main() {
    App::new()
        // MeshPickingPlugin is not a default plugin.
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(
            Update,
            (
                sync_world_camera_viewport,
                draw_mesh_intersections,
                draw_unit_gizmos,
                update_coords_display,
                cancel_drag_on_right_click,
            ),
        )
        // Combat/movement pipeline, chained for a deterministic, easy-to-reason-about order:
        // snapshot everyone's position once, decide/update each unit's behavior off that
        // snapshot, then move and attack. See the "Combat & movement" section below.
        .add_systems(Update, (snapshot_units, evaluate_targets, advance_units, seek_targets, tick_attacks).chain())
        .run();
}

// --- Tuning constants -------------------------------------------------------------------

/// Size (in logical pixels) of a palette icon / a 2D ghost.
const ICON_SIZE: f32 = 56.0;
/// Size (world units) of the cube (Square/Fighter) shape.
const CUBE_SIZE: f32 = 1.0;
/// Radius (world units) of the sphere (Circle/Tank) shape.
const SPHERE_RADIUS: f32 = 0.55;
/// Uniform scale baked into the pyramid (Triangle/Assassin) mesh at creation time (see
/// `setup_scene`), so it sits at roughly the same visual footprint as the (now-smaller)
/// sphere/cube shapes above.
const TRIANGLE_SCALE: f32 = 0.55;
/// Approximate resting height / collision footprint (world units) of the scaled-down pyramid
/// shape -- see the doc comments on `resting_height`/`collision_radius` for why these are
/// hand-picked approximations rather than derived from the mesh.
const TRIANGLE_RESTING_HEIGHT: f32 = 0.35;
const TRIANGLE_COLLISION_RADIUS: f32 = 0.35;
/// Width fraction of each side UI panel. The middle (3D) zone gets `1.0 - 2 * SIDE_FRACTION`.
/// Kept in sync by hand with the `percent(20)` / `percent(60)` literals used in `setup_ui`.
const SIDE_FRACTION: f32 = 0.2;
/// Opacity of every ghost preview (origin marker and destination preview alike).
const GHOST_ALPHA: f32 = 0.5;
/// Shared placement footprint half-size (world units), used for the centerline clamp: a
/// shape's center can get no closer than this to `x = 0` on the wrong side.
const PLACEMENT_HALF_SIZE: f32 = 0.8;
/// Half-extent (world units) of the ground plane on each axis -- matches the `size(30.0, 30.0)`
/// plane in `setup_scene`. A marching unit that reaches the far edge (`+/- MAP_HALF_SIZE` on X)
/// has crossed the whole map and despawns.
const MAP_HALF_SIZE: f32 = 15.0;
/// World units per second of movement, per point of `Stats.speed`. `Stats.speed` is one of
/// 10/20/30 (see `Stats::for_shape`, before per-unit jitter), so this yields roughly 0.9 to 2.7
/// units/sec -- deliberately slower than before so units are easier to watch march/fight.
const UNITS_PER_SPEED_POINT: f32 = 0.09;
/// The `Stats.speed` value (the "++" mid-tier) treated as the 1.0x baseline for attack timing.
const REFERENCE_SPEED: f32 = 20.0;
/// Baseline attack wind-up duration (seconds) at `REFERENCE_SPEED`. Scaled by speed in
/// `windup_duration` so faster units attack more often.
const BASE_WINDUP_SECS: f32 = 0.6;
/// Baseline attack cooldown duration (seconds) at `REFERENCE_SPEED`. Scaled by speed in
/// `cooldown_duration` so faster units attack more often.
const BASE_COOLDOWN_SECS: f32 = 0.8;
/// Step size (world units) between successive rings when searching outward for a free spawn
/// spot around an occupied target -- see `resolve_spawn_position`.
const SPAWN_SEARCH_RING_STEP: f32 = 0.5;
/// How many rings outward `resolve_spawn_position` searches before giving up.
const SPAWN_SEARCH_MAX_RINGS: i32 = 8;
/// How many directions per ring `resolve_spawn_position` samples.
const SPAWN_SEARCH_ANGLE_STEPS: i32 = 8;
/// Lower/upper bound of the random per-unit stat jitter applied once, at spawn time (see
/// `random_stats`): each of a unit's stats is independently multiplied by a factor in this
/// range, so no two units of the same shape are perfectly identical.
const STAT_JITTER_MIN: f32 = 0.85;
const STAT_JITTER_MAX: f32 = 1.15;
/// Width (world units) of a unit's floating health bar (see `draw_unit_gizmos`).
const HEALTH_BAR_WIDTH: f32 = 1.0;
/// Height (world units) above a unit's origin at which its health bar is drawn.
const HEALTH_BAR_HEIGHT_OFFSET: f32 = 1.1;

// --- Marker / data components -------------------------------------------------------------

/// The 3D camera, restricted to the middle 60% of the window.
#[derive(Component)]
struct WorldCamera;

/// The ground plane in the 3D world.
#[derive(Component)]
struct Ground;

/// The text entity that displays the 3D coordinates currently under the cursor.
#[derive(Component)]
struct CoordsDisplay;

/// A permanent, never-moving sidebar "spawn button": drag from one of these to place a unit.
#[derive(Component, Clone, Copy)]
struct PaletteIcon {
    team: Team,
    shape: ShapeKind,
}

/// Marks a permanently placed 3D unit (as opposed to a transient ghost/preview). Also carries
/// the data needed to know how it should behave if it's dragged again.
#[derive(Component, Clone, Copy)]
struct PlacedUnit {
    team: Team,
    shape: ShapeKind,
}

/// Marks a permanently placed *2D* unit: a shape dropped and left sitting in one of the side
/// panels (from the palette, repositioned from another 2D spot, or converted back from a 3D
/// `PlacedUnit`). Parallel to `PlacedUnit`, just for the 2D side of the world. Multiple of
/// these -- and the fixed `PaletteIcon` spawn buttons -- are allowed to overlap freely.
#[derive(Component, Clone, Copy)]
struct Placed2DUnit {
    team: Team,
    shape: ShapeKind,
}

/// Marks the single ephemeral destination-preview entity that exists only while a drag started
/// from the palette (not yet a real placed unit) is in progress.
#[derive(Component)]
struct Ghost;

/// Marks the single ephemeral origin-marker entity: a static reminder of where the current
/// drag started, shown for the whole gesture and removed on drop/cancel.
#[derive(Component)]
struct OriginMarker;

/// A unit's stats. `hp` is also this unit's *maximum* hit points -- `Health` (below) tracks its
/// current, possibly-lower value while it's alive. Each spawned unit gets its own small random
/// jitter applied to these (see `random_stats`), so this is genuinely per-entity, not just a
/// per-shape constant.
#[derive(Component, Clone, Copy)]
struct Stats {
    hp: u32,
    attack: u32,
    speed: u32,
}

impl Stats {
    fn for_shape(shape: ShapeKind) -> Self {
        // A simple +/++/+++ -> 10/20/30 scale, matching the relative ordering from the spec.
        // This is the *baseline* before `random_stats` jitters it for an actual spawned unit.
        match shape {
            ShapeKind::Circle => Stats { hp: 30, attack: 20, speed: 10 }, // Tank
            ShapeKind::Square => Stats { hp: 20, attack: 10, speed: 30 }, // Fighter
            ShapeKind::Triangle => Stats { hp: 10, attack: 30, speed: 20 }, // Assassin
        }
    }
}

/// A tiny, dependency-free xorshift64* PRNG, seeded once at startup. Used only to jitter each
/// newly spawned unit's stats (see `random_stats`) -- nothing here needs to be cryptographically
/// sound, just varied run to run.
#[derive(Resource)]
struct RngState(u64);

impl RngState {
    /// Seeds from the current time so different runs of the app get different jitter; falls
    /// back to a fixed non-zero seed if the clock is somehow unavailable. The `| 1` guarantees
    /// a non-zero, odd seed, which xorshift requires to never get stuck at zero.
    fn seeded() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0x9E3779B97F4A7C15);
        Self(seed | 1)
    }

    /// Next pseudo-random value in `[0.0, 1.0)`.
    fn next_unit_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        // Top 24 bits give us plenty of precision for a f32 in [0, 1).
        ((self.0 >> 40) & 0x00FF_FFFF) as f32 / (1u64 << 24) as f32
    }

    /// Next pseudo-random value in `[min, max)`.
    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_unit_f32() * (max - min)
    }
}

/// Applies a small independent random jitter (`STAT_JITTER_MIN`..`STAT_JITTER_MAX`) to each of
/// `shape`'s baseline `Stats`, so no two spawned units of the same shape are perfectly
/// identical. Called exactly once, the moment a unit is first ever placed (see
/// `on_active_drag_end`'s `is_first_placement` branches) -- repositioning or converting an
/// already-placed unit reuses its existing (already-jittered) `Stats` instead of rolling new
/// ones.
fn random_stats(shape: ShapeKind, rng: &mut RngState) -> Stats {
    let base = Stats::for_shape(shape);
    let jitter = |value: u32, rng: &mut RngState| -> u32 { (value as f32 * rng.next_range(STAT_JITTER_MIN, STAT_JITTER_MAX)).round().max(1.0) as u32 };
    Stats {
        hp: jitter(base.hp, rng),
        attack: jitter(base.attack, rng),
        speed: jitter(base.speed, rng),
    }
}

/// Current HP of a live battlefield unit. Attached (at that unit's own jittered `Stats.hp`)
/// whenever an entity enters its 3D `PlacedUnit` form, and removed when it leaves the
/// battlefield (converted back to a 2D unit) -- a unit that's redeployed later starts fresh, not
/// wherever it left off.
#[derive(Component)]
struct Health(f32);

/// A unit's detection/attack radii, entirely independent of its own physical footprint (see
/// `collision_radius`, which is a different, spawn-collision-only concept and must never be
/// mixed in here).
#[derive(Component, Clone, Copy)]
struct CombatRanges {
    detection: f32,
    attack: f32,
}

/// Default behavior: marching in a straight line toward the enemy camp. Every live battlefield
/// unit has exactly one of `Advancing`/`Seeking`/`Attacking` at a time -- this is the ECS
/// "state machine via component" pattern: changing behavior means removing whichever of these
/// is currently present and inserting a different one, rather than mutating a single global
/// `State` enum field.
#[derive(Component)]
struct Advancing;

/// Moving in a straight line (diagonal allowed) directly toward `target`, which is within
/// `CombatRanges.detection` but not yet within `CombatRanges.attack`.
#[derive(Component)]
struct Seeking {
    target: Entity,
}

/// Stopped, attacking `target` (within `CombatRanges.attack`). `phase`/`timer` alternate
/// between a wind-up (before a hit lands) and a cooldown (after, before the next wind-up can
/// start) -- see `tick_attacks`.
#[derive(Component)]
struct Attacking {
    target: Entity,
    phase: AttackPhase,
    timer: Timer,
}

#[derive(Clone, Copy, PartialEq)]
enum AttackPhase {
    WindUp,
    Cooldown,
}

/// Which team a unit belongs to. Fixed for a palette icon's whole lifetime, and copied onto
/// every unit it spawns.
///
/// Note this is deliberately *not* a `Component`: it only ever lives as a field inside
/// `PaletteIcon`/`PlacedUnit`/`Placed2DUnit`, which are the single source of truth for a
/// unit's team. Deriving `Component` here would let a system query `&Team` and compile
/// perfectly while silently matching zero entities.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum Team {
    #[default]
    Blue,
    Red,
}

/// Which of the three unit archetypes a palette icon (and whatever it spawns) represents.
/// Not a `Component`, for the same reason as `Team` above.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum ShapeKind {
    #[default]
    Circle,
    Square,
    Triangle,
}

/// Which of the three horizontal screen zones a given x position falls into.
#[derive(Clone, Copy, PartialEq)]
enum Zone {
    LeftUi,
    World,
    RightUi,
}

fn zone_of(x: f32, window_width: f32) -> Zone {
    let (left_boundary, right_boundary) = zone_boundaries(window_width);
    if x < left_boundary {
        Zone::LeftUi
    } else if x > right_boundary {
        Zone::RightUi
    } else {
        Zone::World
    }
}

fn zone_boundaries(window_width: f32) -> (f32, f32) {
    (window_width * SIDE_FRACTION, window_width * (1.0 - SIDE_FRACTION))
}

/// Clamps a raw ground-plane x coordinate to `team`'s half of the world, leaving a gap of
/// `PLACEMENT_HALF_SIZE` around the centerline so a clamped shape doesn't visually poke across.
fn clamp_to_team_half(x: f32, team: Team) -> f32 {
    match team {
        Team::Blue => x.min(-PLACEMENT_HALF_SIZE),
        Team::Red => x.max(PLACEMENT_HALF_SIZE),
    }
}

/// Where a shape's origin should sit above `y = 0` so it rests on the ground.
fn resting_height(shape: ShapeKind) -> f32 {
    match shape {
        ShapeKind::Circle => SPHERE_RADIUS,
        ShapeKind::Square => CUBE_SIZE / 2.0,
        // Bevy's default `Tetrahedron` doesn't expose a simple size to compute this from
        // exactly; this is a reasonable approximation for the `TRIANGLE_SCALE`-scaled mesh --
        // nudge it if it looks off.
        ShapeKind::Triangle => TRIANGLE_RESTING_HEIGHT,
    }
}

/// A shape's own physical footprint radius, used *only* to keep spawns from overlapping (see
/// `is_occupied_at`/`resolve_spawn_position`). Deliberately simple approximations, in the same
/// spirit as `resting_height` -- and deliberately never reused for `CombatRanges`, which is an
/// unrelated, gameplay-facing concept.
fn collision_radius(shape: ShapeKind) -> f32 {
    match shape {
        ShapeKind::Circle => SPHERE_RADIUS,
        ShapeKind::Square => CUBE_SIZE / 2.0,
        ShapeKind::Triangle => TRIANGLE_COLLISION_RADIUS,
    }
}

/// A unit's detection/attack radii by role. Placeholder, narratively-flavored values (the Tank
/// has to close the distance before it can swing; the Assassin needs to get in close but has
/// the speed to do it) -- not a balanced design, same spirit as `Stats::for_shape`.
fn combat_ranges(shape: ShapeKind) -> CombatRanges {
    match shape {
        ShapeKind::Circle => CombatRanges { detection: 6.0, attack: 1.2 },
        ShapeKind::Square => CombatRanges { detection: 5.0, attack: 1.4 },
        ShapeKind::Triangle => CombatRanges { detection: 4.0, attack: 1.0 },
    }
}

/// World units per second of movement for a unit with this `Stats.speed`.
fn move_speed(speed: u32) -> f32 {
    speed as f32 * UNITS_PER_SPEED_POINT
}

/// Attack wind-up duration for a unit with this `Stats.speed` -- shorter for faster units, so
/// they attack more often. Simple inverse-of-speed scaling around `REFERENCE_SPEED`; tune the
/// two `BASE_*_SECS` constants (or this formula) freely, nothing else depends on its shape.
fn windup_duration(speed: u32) -> f32 {
    BASE_WINDUP_SECS * (REFERENCE_SPEED / speed as f32)
}

/// Attack cooldown duration for a unit with this `Stats.speed` -- same inverse-speed scaling as
/// `windup_duration`.
fn cooldown_duration(speed: u32) -> f32 {
    BASE_COOLDOWN_SECS * (REFERENCE_SPEED / speed as f32)
}

/// Whether a candidate ground spot at `(x, z)` -- for a unit with footprint radius
/// `my_radius` -- overlaps any existing placed unit's own footprint. Shape-aware on both
/// sides: two big units need more clearance between them than two small ones.
fn is_occupied_at(x: f32, z: f32, my_radius: f32, units: &Query<(&Transform, &PlacedUnit)>) -> bool {
    units.iter().any(|(transform, unit)| {
        let delta = Vec2::new(transform.translation.x - x, transform.translation.z - z);
        delta.length() < my_radius + collision_radius(unit.shape)
    })
}

/// Finds where a unit of `shape` for `team` should actually spawn/land, starting from a
/// desired `(x, z)` and never crossing the centerline. If the desired spot is free, that's the
/// answer. Otherwise this searches outward in rings for the nearest free spot, still strictly
/// within `team`'s half. Returns `None` if nothing reasonably close is free -- in which case
/// the drop should be refused. Shared by the ghost preview and the actual drop (see
/// `on_active_drag`/`on_active_drag_end`) so they always agree on exactly where a unit will
/// land.
fn resolve_spawn_position(desired_x: f32, desired_z: f32, team: Team, shape: ShapeKind, units: &Query<(&Transform, &PlacedUnit)>) -> Option<Vec3> {
    let my_radius = collision_radius(shape);
    let y = resting_height(shape);

    let base_x = clamp_to_team_half(desired_x, team);
    if !is_occupied_at(base_x, desired_z, my_radius, units) {
        return Some(Vec3::new(base_x, y, desired_z));
    }

    for ring in 1..=SPAWN_SEARCH_MAX_RINGS {
        let radius = ring as f32 * SPAWN_SEARCH_RING_STEP;
        for step in 0..SPAWN_SEARCH_ANGLE_STEPS {
            let angle = (step as f32 / SPAWN_SEARCH_ANGLE_STEPS as f32) * std::f32::consts::TAU;
            let candidate_x = clamp_to_team_half(desired_x + radius * angle.cos(), team);
            let candidate_z = desired_z + radius * angle.sin();
            if !is_occupied_at(candidate_x, candidate_z, my_radius, units) {
                return Some(Vec3::new(candidate_x, y, candidate_z));
            }
        }
    }
    None
}

fn role_name(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Circle => "Tank",
        ShapeKind::Square => "Fighter",
        ShapeKind::Triangle => "Assassin",
    }
}

fn stats_label(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Circle => "HP +++   ATK ++    SPD +",
        ShapeKind::Square => "HP ++    ATK +     SPD +++",
        ShapeKind::Triangle => "HP +     ATK +++   SPD ++",
    }
}

/// The Node styling that makes a 2D icon/ghost/origin-marker box actually *look like* `shape`,
/// in a `size` x `size` footprint, tinted `color`.
///
/// Circle and Square are plain boxes (a full `BorderRadius` for the circle, none for the
/// square). Triangle uses the classic zero-size / colored-border trick instead of relying on a
/// glyph: a box with `width: 0, height: 0` whose *border* is the only visible thing -- equal
/// transparent left/right borders and a `color`-filled bottom border of the box's full height
/// draw an upward-pointing triangle inscribed in the same `size` x `size` footprint every other
/// icon uses (Bevy's default `BoxSizing::BorderBox` means the border is what actually occupies
/// the layout space here, not the declared 0 content size). This sidesteps needing the default
/// UI font to contain geometric-shape glyphs (●/■/▲), which it very likely doesn't -- that
/// mismatch, not any layout bug, is why the Assassin icon rendered as a plain, glyph-less
/// square.
struct IconVisual {
    width: Val,
    height: Val,
    border: UiRect,
    border_radius: BorderRadius,
    background: Color,
    border_color: BorderColor,
}

fn icon_visual(shape: ShapeKind, size: f32, color: Color) -> IconVisual {
    match shape {
        ShapeKind::Circle => IconVisual {
            width: Val::Px(size),
            height: Val::Px(size),
            border: UiRect::ZERO,
            border_radius: BorderRadius::all(px(999)),
            background: color,
            border_color: BorderColor::DEFAULT,
        },
        ShapeKind::Square => IconVisual {
            width: Val::Px(size),
            height: Val::Px(size),
            border: UiRect::ZERO,
            border_radius: BorderRadius::default(),
            background: color,
            border_color: BorderColor::DEFAULT,
        },
        ShapeKind::Triangle => {
            // Same visual trick as before (transparent left/right borders, a colored bottom
            // border), but with an *explicit* width/height equal to every other icon's,
            // instead of leaving Taffy to derive the box's size purely from oversized borders
            // on a declared-0 box. The content box still resolves to 0 either way (the borders
            // alone already sum to `size` on both axes) so the visual result is identical, but
            // this removes the one structural difference between the Triangle icon and every
            // other shape's `Node` -- which was the leading suspect for a triangle-only bug.
            let half = size / 2.0;
            IconVisual {
                width: Val::Px(size),
                height: Val::Px(size),
                border: UiRect::new(px(half), px(half), Val::Px(0.0), px(size)),
                border_radius: BorderRadius::default(),
                background: Color::NONE,
                border_color: BorderColor { top: Color::NONE, right: Color::NONE, bottom: color, left: Color::NONE },
            }
        }
    }
}

// --- Shared resources ---------------------------------------------------------------------

/// Mesh/material handles, created once and reused everywhere so repeated drags don't leak new
/// assets.
#[derive(Resource)]
struct SharedAssets {
    sphere_mesh: Handle<Mesh>,
    cube_mesh: Handle<Mesh>,
    pyramid_mesh: Handle<Mesh>,
    blue_solid: Handle<StandardMaterial>,
    blue_ghost_3d: Handle<StandardMaterial>,
    red_solid: Handle<StandardMaterial>,
    red_ghost_3d: Handle<StandardMaterial>,
}

impl SharedAssets {
    fn mesh(&self, shape: ShapeKind) -> Handle<Mesh> {
        match shape {
            ShapeKind::Circle => self.sphere_mesh.clone(),
            ShapeKind::Square => self.cube_mesh.clone(),
            ShapeKind::Triangle => self.pyramid_mesh.clone(),
        }
    }

    fn solid_material(&self, team: Team) -> Handle<StandardMaterial> {
        match team {
            Team::Blue => self.blue_solid.clone(),
            Team::Red => self.red_solid.clone(),
        }
    }

    /// Lighter, translucent version of `team`'s color -- used for the destination preview.
    fn ghost_material_3d(&self, team: Team) -> Handle<StandardMaterial> {
        match team {
            Team::Blue => self.blue_ghost_3d.clone(),
            Team::Red => self.red_ghost_3d.clone(),
        }
    }

    /// Light, translucent UI color for a 2D ghost, team-tinted -- used for the destination
    /// preview while it's in its 2D form.
    fn ghost_ui_color(&self, team: Team) -> Color {
        match team {
            Team::Blue => Color::from(BLUE_300).with_alpha(GHOST_ALPHA),
            Team::Red => Color::from(RED_300).with_alpha(GHOST_ALPHA),
        }
    }

    /// Opaque, team-tinted UI color -- the 2D counterpart to `solid_material`, used for a
    /// resting (not being dragged) placed 2D unit.
    fn solid_ui_color(&self, team: Team) -> Color {
        match team {
            Team::Blue => Color::from(BLUE_500),
            Team::Red => Color::from(RED_500),
        }
    }

    /// Fixed translucent blue used for the *origin* marker, regardless of which team is
    /// dragging -- this is what visually distinguishes "where it started" from "where it's
    /// going" (which is always tinted in the team's own color instead).
    fn origin_material_3d(&self) -> Handle<StandardMaterial> {
        self.blue_ghost_3d.clone()
    }

    fn origin_ui_color(&self) -> Color {
        Color::from(BLUE_300).with_alpha(GHOST_ALPHA)
    }
}

/// Which kind of entity a drag started from -- determines what happens on cancel, and whether
/// a fresh-placement occupancy check applies.
#[derive(Clone, Copy, PartialEq, Default)]
enum DragOrigin {
    /// Started from a permanent sidebar spawn button. Nothing existed before the drag, so a
    /// cancelled or rejected drop just discards the ghost.
    #[default]
    Palette,
    /// Started from an already-placed 3D unit. A cancelled drop, or a rejected (occupied) 3D
    /// placement, must restore that unit to its exact original transform.
    PlacedUnit,
    /// Started from an already-placed 2D unit (sitting in a side panel). A cancelled drop, or
    /// a rejected (occupied) 3D placement, must restore it to its exact original screen
    /// position.
    Placed2D,
}

/// Bookkeeping for the in-progress drag, if any. `moving` is the single source of truth for
/// whether a drag is active: `Some` means a drag (and its moving entity) is active, `None`
/// means it isn't.
#[derive(Resource, Default)]
struct DragSession {
    /// The entity currently following the cursor: a fresh `Ghost` for a palette-started drag,
    /// or the placed unit itself for a unit-started drag.
    moving: Option<Entity>,
    /// The static origin-marker entity, shown for the whole gesture at the starting position.
    origin_marker: Option<Entity>,
    origin: DragOrigin,
    team: Team,
    shape: ShapeKind,
    /// Only meaningful when `origin == PlacedUnit`: the unit's transform before this drag
    /// began, used both to place the origin marker and to restore on cancellation.
    original_transform: Transform,
    /// Only meaningful when `origin == Placed2D`: the unit's on-screen center before this drag
    /// began, used both to place the origin marker and to restore on cancellation.
    original_screen_pos: Vec2,
}

// --- Combat & movement -----------------------------------------------------------------
//
// Every live battlefield unit (a `PlacedUnit` with `Health`/`CombatRanges`/one of
// `Advancing`/`Seeking`/`Attacking`) goes through this chained pipeline every frame:
//
// 1. `snapshot_units` reads everyone's (entity, position, team) into a plain `UnitSnapshot`
//    resource. Every other system below reads positions from *this*, not from a live query --
//    that's what lets `evaluate_targets` (which needs to see every unit) and `seek_targets`
//    (which needs to move its own unit while reading its target's position) coexist without
//    fighting over mutable/immutable access to `Transform`, no `ParamSet` needed.
// 2. `evaluate_targets` re-derives, from scratch, what each unit's behavior *should* be this
//    frame (Attacking > Seeking > Advancing, in that priority) and swaps components only when
//    it actually needs to change. Because this always recomputes fresh off the snapshot, target
//    death, a target leaving range, or a closer enemy showing up are all just naturally handled
//    every frame -- there's no separate "did my target die" bookkeeping to get out of sync.
// 3. `advance_units`/`seek_targets` move whichever units are in that respective state.
// 4. `tick_attacks` ticks wind-up/cooldown timers and applies damage.

/// A plain per-frame cache of every battlefield unit's (entity, position, team), rebuilt by
/// `snapshot_units`. Everything downstream reads positions from here instead of a live query,
/// which is what keeps the mutable-Transform systems below simple (no `ParamSet` needed for
/// them) while still letting them see every other unit's position.
#[derive(Resource, Default)]
struct UnitSnapshot {
    units: Vec<(Entity, Vec3, Team)>,
}

fn snapshot_units(mut snapshot: ResMut<UnitSnapshot>, units: Query<(Entity, &Transform, &PlacedUnit)>, session: Res<DragSession>) {
    snapshot.units.clear();
    snapshot.units.extend(
        units
            .iter()
            // A unit being dragged keeps its `PlacedUnit`/`Transform` for the whole gesture, so
            // without this it would stay targetable while the player holds it -- and could be
            // killed mid-drag, leaving `DragSession.moving` pointing at a despawned entity.
            // Its own behavior is already paused (see `on_unit_drag_start`); this is the other
            // half of that, making it invisible to everyone else's targeting too.
            .filter(|(entity, _, _)| session.moving != Some(*entity))
            .map(|(entity, transform, unit)| (entity, transform.translation, unit.team)),
    );
}

/// Re-derives each unit's behavior from scratch every frame: the closest enemy within
/// `CombatRanges.attack` (if any) beats the closest within `CombatRanges.detection` (if any)
/// beats plain `Advancing`. Only actually swaps components when the outcome differs from the
/// unit's current state, so a unit that's still correctly attacking/seeking the same target
/// keeps its `Attacking` timer/phase untouched.
fn evaluate_targets(
    mut commands: Commands,
    snapshot: Res<UnitSnapshot>,
    self_units: Query<(Entity, &Transform, &PlacedUnit, &Stats, &CombatRanges, Has<Advancing>, Option<&Seeking>, Option<&Attacking>)>,
) {
    for (entity, transform, unit, stats, ranges, is_advancing, seeking, attacking) in &self_units {
        let mut closest_attack: Option<(Entity, f32)> = None;
        let mut closest_detect: Option<(Entity, f32)> = None;
        for &(other, other_pos, other_team) in &snapshot.units {
            if other == entity || other_team == unit.team {
                continue;
            }
            let distance = transform.translation.distance(other_pos);
            if distance <= ranges.attack && closest_attack.map_or(true, |(_, d)| distance < d) {
                closest_attack = Some((other, distance));
            }
            if distance <= ranges.detection && closest_detect.map_or(true, |(_, d)| distance < d) {
                closest_detect = Some((other, distance));
            }
        }

        if let Some((target, _)) = closest_attack {
            if !attacking.is_some_and(|a| a.target == target) {
                commands.entity(entity).remove::<(Advancing, Seeking, Attacking)>().insert(Attacking {
                    target,
                    phase: AttackPhase::WindUp,
                    timer: Timer::from_seconds(windup_duration(stats.speed), TimerMode::Once),
                });
            }
        } else if let Some((target, _)) = closest_detect {
            if !seeking.is_some_and(|s| s.target == target) {
                commands.entity(entity).remove::<(Advancing, Seeking, Attacking)>().insert(Seeking { target });
            }
        } else if !is_advancing {
            commands.entity(entity).remove::<(Advancing, Seeking, Attacking)>().insert(Advancing);
        }
    }
}

/// Default behavior: march in a straight line along the team's forward axis (X only -- no Z
/// drift) at a speed derived from `Stats.speed`, framerate-independent via `delta_secs`.
/// Despawns (after logging) a unit that reaches the far edge of the map.
fn advance_units(mut commands: Commands, mut units: Query<(Entity, &mut Transform, &PlacedUnit, &Stats), With<Advancing>>, time: Res<Time>) {
    let dt = time.delta_secs();
    for (entity, mut transform, unit, stats) in &mut units {
        let team = unit.team;
        let forward_x = match team {
            Team::Blue => 1.0,
            Team::Red => -1.0,
        };
        transform.translation.x += forward_x * move_speed(stats.speed) * dt;

        let reached_far_edge = match team {
            Team::Blue => transform.translation.x >= MAP_HALF_SIZE,
            Team::Red => transform.translation.x <= -MAP_HALF_SIZE,
        };
        if reached_far_edge {
            info!("{team:?} unit {entity:?} reached the far edge of the map and was removed.");
            commands.entity(entity).despawn();
        }
    }
}

/// `Seeking` behavior: move in a straight line (diagonal allowed) directly toward the target's
/// *current* position (read from `UnitSnapshot`, not a live query -- see the section doc
/// comment above), without overshooting past it.
fn seek_targets(mut units: Query<(&mut Transform, &Stats, &Seeking)>, snapshot: Res<UnitSnapshot>, time: Res<Time>) {
    let dt = time.delta_secs();
    for (mut transform, stats, seeking) in &mut units {
        let Some(&(_, target_pos, _)) = snapshot.units.iter().find(|(e, _, _)| *e == seeking.target) else {
            // Target despawned this exact frame; evaluate_targets will pick a new behavior on
            // the next one.
            continue;
        };
        let to_target = Vec3::new(target_pos.x - transform.translation.x, 0.0, target_pos.z - transform.translation.z);
        let distance = to_target.length();
        let step = move_speed(stats.speed) * dt;
        if distance <= step || distance < 1e-4 {
            transform.translation.x = target_pos.x;
            transform.translation.z = target_pos.z;
        } else {
            let dir = to_target / distance;
            transform.translation.x += dir.x * step;
            transform.translation.z += dir.z * step;
        }
    }
}

/// `Attacking` behavior: while stopped, alternates the wind-up/cooldown timer. Damage lands at
/// the end of the wind-up (if the target's still alive), then cooldown blocks the next wind-up
/// from starting immediately. A dead target is despawned right here, so the next frame's
/// `snapshot_units` (and therefore `evaluate_targets`) already sees a world without it.
fn tick_attacks(mut commands: Commands, mut attackers: Query<(&Stats, &mut Attacking)>, mut healths: Query<&mut Health>, time: Res<Time>) {
    let dt = time.delta();
    for (stats, mut attacking) in &mut attackers {
        attacking.timer.tick(dt);
        if !attacking.timer.is_finished() {
            continue;
        }
        match attacking.phase {
            AttackPhase::WindUp => {
                if let Ok(mut health) = healths.get_mut(attacking.target) {
                    health.0 -= stats.attack as f32;
                    if health.0 <= 0.0 {
                        commands.entity(attacking.target).despawn();
                    }
                }
                attacking.phase = AttackPhase::Cooldown;
                attacking.timer = Timer::from_seconds(cooldown_duration(stats.speed), TimerMode::Once);
            }
            AttackPhase::Cooldown => {
                attacking.phase = AttackPhase::WindUp;
                attacking.timer = Timer::from_seconds(windup_duration(stats.speed), TimerMode::Once);
            }
        }
    }
}

/// Debug overlay for every live 3D `PlacedUnit`: its detection/attack `CombatRanges` drawn as
/// flat circles on the ground (translucent yellow / red), plus a small floating health bar
/// (a dark background sliver with a green -> yellow -> red fill sized to its current HP
/// fraction) hovering just above it.
fn draw_unit_gizmos(units: Query<(&Transform, &Health, &Stats, &CombatRanges), With<PlacedUnit>>, mut gizmos: Gizmos) {
    // A rotation that lays a gizmo circle flat on the XZ ground plane instead of the default
    // vertical XY orientation.
    let ground_facing = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);

    for (transform, health, stats, ranges) in &units {
        let pos = transform.translation;

        gizmos.circle(Isometry3d::new(Vec3::new(pos.x, 0.02, pos.z), ground_facing), ranges.detection, Color::from(YELLOW_300).with_alpha(0.35));
        gizmos.circle(Isometry3d::new(Vec3::new(pos.x, 0.03, pos.z), ground_facing), ranges.attack, Color::from(RED_400).with_alpha(0.55));

        let bar_y = pos.y + HEALTH_BAR_HEIGHT_OFFSET;
        let half_width = HEALTH_BAR_WIDTH / 2.0;
        let left = Vec3::new(pos.x - half_width, bar_y, pos.z);
        let right = Vec3::new(pos.x + half_width, bar_y, pos.z);
        gizmos.line(left, right, Color::BLACK.with_alpha(0.6));

        let fraction = (health.0 / stats.hp as f32).clamp(0.0, 1.0);
        let fill_color = if fraction > 0.5 {
            Color::from(GREEN_500)
        } else if fraction > 0.25 {
            Color::from(YELLOW_400)
        } else {
            Color::from(RED_500)
        };
        let fill_right = Vec3::new(left.x + HEALTH_BAR_WIDTH * fraction, bar_y + 0.015, pos.z);
        gizmos.line(left + Vec3::Y * 0.015, fill_right, fill_color);
    }
}

// --- Scene setup (3D world) ---------------------------------------------------------------

fn setup_scene(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Ground plane, split conceptually at x = 0 into Blue's half (x < 0) and Red's half
    // (x > 0).
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(30.0, 30.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(GRAY_300))),
        Ground,
    ));

    // Light.
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // World camera. Its `viewport` starts empty and is filled in every frame by
    // `sync_world_camera_viewport`, which keeps it locked to exactly the middle 60% of the
    // window -- so the ground plane below renders centered on that strip.
    commands.spawn((
        WorldCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Shared meshes/materials for all placed units and ghosts. The pyramid mesh has
    // `TRIANGLE_SCALE` baked directly into its vertices here (rather than applied as a
    // per-entity `Transform` scale later) so every spot that builds a unit's `Transform` from a
    // plain translation keeps working unchanged.
    let sphere_mesh = meshes.add(Sphere::new(SPHERE_RADIUS).mesh().ico(5).unwrap());
    let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
    // `scaled_by` lives on the finished `Mesh`, not on the shape's mesh *builder*, so convert
    // first and scale after.
    let pyramid_mesh = meshes.add(Mesh::from(Tetrahedron::default()).scaled_by(Vec3::splat(TRIANGLE_SCALE)));

    let blue_solid = materials.add(Color::from(BLUE_500));
    let mut blue_ghost_mat: StandardMaterial = Color::from(BLUE_300).with_alpha(GHOST_ALPHA).into();
    blue_ghost_mat.alpha_mode = AlphaMode::Blend;
    let blue_ghost_3d = materials.add(blue_ghost_mat);

    let red_solid = materials.add(Color::from(RED_500));
    let mut red_ghost_mat: StandardMaterial = Color::from(RED_300).with_alpha(GHOST_ALPHA).into();
    red_ghost_mat.alpha_mode = AlphaMode::Blend;
    let red_ghost_3d = materials.add(red_ghost_mat);

    commands.insert_resource(SharedAssets {
        sphere_mesh,
        cube_mesh,
        pyramid_mesh,
        blue_solid,
        blue_ghost_3d,
        red_solid,
        red_ghost_3d,
    });
    commands.insert_resource(DragSession::default());
    commands.insert_resource(UnitSnapshot::default());
    commands.insert_resource(RngState::seeded());
}

/// Keeps the world camera's viewport locked to exactly the middle 60% of the window, in
/// physical pixels, every frame (cheap, and trivially correct across window resizes).
fn sync_world_camera_viewport(windows: Query<&Window>, mut world_camera: Single<&mut Camera, With<WorldCamera>>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let phys_width = window.physical_width();
    let phys_height = window.physical_height();
    let left = (phys_width as f32 * SIDE_FRACTION) as u32;
    let right = (phys_width as f32 * (1.0 - SIDE_FRACTION)) as u32;

    world_camera.viewport = Some(Viewport {
        physical_position: UVec2::new(left, 0),
        physical_size: UVec2::new(right - left, phys_height),
        ..default()
    });
}

/// Live readout of the 3D coordinates under the cursor, refreshed every frame. Only shows a
/// value while the cursor is actually over the middle (3D) zone.
fn update_coords_display(
    windows: Query<&Window>,
    world_camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    mut text_query: Query<&mut Text, With<CoordsDisplay>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        *text = Text::new("Move the cursor over\nthe 3D area.");
        return;
    };
    if zone_of(cursor.x, window.width()) != Zone::World {
        *text = Text::new("Move the cursor over\nthe 3D area.");
        return;
    }

    let (camera, camera_transform) = *world_camera;
    match point_on_ground(camera, camera_transform, cursor) {
        Some(point) => {
            *text = Text::new(format!("x = {:.2}\ny = {:.2}\nz = {:.2}", point.x, point.y, point.z));
        }
        None => *text = Text::new("Move the cursor over\nthe 3D area."),
    }
}

/// Same hit-indicator gizmo as Bevy's mesh_picking example: a small red sphere plus a pink
/// arrow at the nearest pointer/mesh intersection, for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

// --- UI setup --------------------------------------------------------------------------

fn setup_ui(mut commands: Commands) {
    // UI camera: full window, no viewport, renders after (on top of) the world camera. Every
    // UI root node below has no explicit target, so it attaches to this camera automatically
    // thanks to `IsDefaultUiCamera` -- which is also why UI coordinates stay in plain,
    // un-clipped window-space pixels regardless of the world camera's own viewport.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        IsDefaultUiCamera,
    ));

    spawn_side_panel(&mut commands, Team::Blue, false);
    spawn_side_panel(&mut commands, Team::Red, true);

    // Coordinates HUD, anchored to the top-right corner of the middle (3D) zone -- i.e. its
    // right edge sits exactly on the world/right-panel boundary.
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: px(12),
            right: percent(20), // keep in sync with SIDE_FRACTION: right panel width
            display: Display::Flex,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },))
        .with_children(|corner| {
            corner
                .spawn((
                    Node {
                        padding: UiRect::all(px(8)),
                        margin: UiRect::right(px(12)),
                        min_width: px(140),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK.with_alpha(0.6)),
                ))
                .with_children(|hud| {
                    hud.spawn((
                        Text::new("Move the cursor over\nthe 3D area."),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        CoordsDisplay,
                    ));
                });
        });
}

/// Builds one side panel: a title, one row per shape (icon + role name + stat readout), and a
/// short instructions line at the bottom.
fn spawn_side_panel(commands: &mut Commands, team: Team, anchor_right: bool) {
    let team_color = match team {
        Team::Blue => Color::from(BLUE_500),
        Team::Red => Color::from(RED_500),
    };
    let title = match team {
        Team::Blue => "Team Blue (left)",
        Team::Red => "Team Red (right)",
    };

    let mut panel_node = Node {
        position_type: PositionType::Absolute,
        top: px(0),
        width: percent(20), // keep in sync with SIDE_FRACTION
        height: percent(100),
        flex_direction: FlexDirection::Column,
        row_gap: px(16),
        padding: UiRect::all(px(12)),
        ..default()
    };
    if anchor_right {
        panel_node.right = px(0);
    } else {
        panel_node.left = px(0);
    }

    commands.spawn((panel_node, BackgroundColor(Color::BLACK))).with_children(|panel| {
        panel.spawn((Text::new(title), TextColor(Color::WHITE)));

        for shape in [ShapeKind::Circle, ShapeKind::Square, ShapeKind::Triangle] {
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|row| {
                    let visual = icon_visual(shape, ICON_SIZE, team_color);
                    row.spawn((
                        PaletteIcon { team, shape },
                        Node {
                            width: visual.width,
                            height: visual.height,
                            border: visual.border,
                            border_radius: visual.border_radius,
                            ..default()
                        },
                        BackgroundColor(visual.background),
                        visual.border_color,
                    ))
                    .observe(on_palette_drag_start)
                    .observe(on_active_drag)
                    .observe(on_active_drag_end);

                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    })
                    .with_children(|label| {
                        label.spawn((Text::new(role_name(shape)), TextColor(Color::WHITE)));
                        label.spawn((
                            Text::new(stats_label(shape)),
                            TextColor(Color::from(GRAY_300)),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                        ));
                    });
                });
        }

        panel.spawn((
            Text::new(
                "Drag a shape into the middle to place it on your half.\n\
                 Drag a placed unit to move it, or back onto a panel to remove it.\n\
                 Right-click during a drag to cancel it.",
            ),
            TextColor(Color::from(GRAY_300)),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
        ));
    });
}

// --- The drag / placement logic ------------------------------------------------------------

/// Casts a ray from the world camera through a window-space point and intersects it with the
/// y = 0 ground plane, returning the resulting world-space point. `window_pos` is a plain
/// window-space (not viewport-space) coordinate; `viewport_to_world` already accounts for the
/// world camera's own viewport offset internally.
fn point_on_ground(camera: &Camera, camera_transform: &GlobalTransform, window_pos: Vec2) -> Option<Vec3> {
    let ray = camera.viewport_to_world(camera_transform, window_pos).ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    Some(ray.get_point(distance))
}

/// Spawns the static origin marker for the current drag: a translucent, fixed-blue reminder of
/// where the drag started, shown for the whole gesture. `as_3d` picks its initial (and only,
/// since it never moves) representation.
fn spawn_origin_marker(
    commands: &mut Commands,
    assets: &SharedAssets,
    shape: ShapeKind,
    screen_pos: Vec2,
    world_pos: Option<Vec3>,
) -> Entity {
    if let Some(world_pos) = world_pos {
        commands
            .spawn((
                OriginMarker,
                Mesh3d(assets.mesh(shape)),
                MeshMaterial3d(assets.origin_material_3d()),
                Transform::from_translation(world_pos),
                Pickable::IGNORE,
            ))
            .id()
    } else {
        let visual = icon_visual(shape, ICON_SIZE, assets.origin_ui_color());
        commands
            .spawn((
                OriginMarker,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(screen_pos.x - ICON_SIZE / 2.0),
                    top: Val::Px(screen_pos.y - ICON_SIZE / 2.0),
                    width: visual.width,
                    height: visual.height,
                    border: visual.border,
                    border_radius: visual.border_radius,
                    ..default()
                },
                BackgroundColor(visual.background),
                visual.border_color,
                GlobalZIndex(19),
                Pickable::IGNORE,
            ))
            .id()
    }
}

/// Starts a new drag from a palette icon: records the icon's fixed team/shape for the rest of
/// the session, spawns the (initially 2D) destination ghost centered on the cursor, and spawns
/// a matching 2D origin marker at the same spot.
fn on_palette_drag_start(
    trigger: On<Pointer<DragStart>>,
    icons: Query<(&PaletteIcon, &UiGlobalTransform)>,
    mut commands: Commands,
    assets: Res<SharedAssets>,
    mut session: ResMut<DragSession>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    let Ok((&icon, icon_transform)) = icons.get(trigger.entity) else {
        return;
    };

    session.team = icon.team;
    session.shape = icon.shape;
    session.origin = DragOrigin::Palette;

    let pos = trigger.pointer_location.position;
    let visual = icon_visual(icon.shape, ICON_SIZE, assets.ghost_ui_color(icon.team));
    let ghost = commands
        .spawn((
            Ghost,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pos.x - ICON_SIZE / 2.0),
                top: Val::Px(pos.y - ICON_SIZE / 2.0),
                width: visual.width,
                height: visual.height,
                border: visual.border,
                border_radius: visual.border_radius,
                ..default()
            },
            BackgroundColor(visual.background),
            visual.border_color,
            GlobalZIndex(20),
            Pickable::IGNORE,
        ))
        .id();

    session.moving = Some(ghost);
    // Anchor the origin marker to the icon's actual on-screen center (`UiGlobalTransform`'s
    // translation), not the raw click point -- the click can land off-center within the icon,
    // which is exactly what made the marker look like a misaligned, "glitched" partial overlay
    // rather than a clean highlight sitting on top of the icon.
    let icon_center = icon_transform.translation;
    session.origin_marker = Some(spawn_origin_marker(&mut commands, &assets, icon.shape, icon_center, None));
}

/// Starts a drag on an already-placed unit: records its team/shape/transform, switches the
/// unit itself to its translucent "being dragged" material (it becomes the moving/destination
/// preview for the rest of the gesture), and spawns a 3D origin marker at its original spot.
fn on_unit_drag_start(
    trigger: On<Pointer<DragStart>>,
    units: Query<(&Transform, &PlacedUnit)>,
    mut commands: Commands,
    assets: Res<SharedAssets>,
    mut session: ResMut<DragSession>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    let Ok((&transform, &unit)) = units.get(trigger.entity) else {
        return;
    };

    session.team = unit.team;
    session.shape = unit.shape;
    session.origin = DragOrigin::PlacedUnit;
    session.original_transform = transform;
    session.moving = Some(trigger.entity);

    // Swap to the translucent team-colored preview material; the mesh/entity/observers stay
    // exactly the same, so it keeps receiving Drag/DragEnd events for the rest of the gesture.
    // Also pause combat/movement for the duration of the drag: whichever behavior state it was
    // in gets removed, so the combat systems (all scoped to Advancing/Seeking/Attacking) simply
    // stop touching this entity until it's re-placed.
    commands
        .entity(trigger.entity)
        .insert(MeshMaterial3d(assets.ghost_material_3d(unit.team)))
        .remove::<(Advancing, Seeking, Attacking)>();

    session.origin_marker = Some(spawn_origin_marker(&mut commands, &assets, unit.shape, Vec2::ZERO, Some(transform.translation)));
}

/// Starts a drag on an already-placed *2D* unit: parallel to `on_unit_drag_start`, but the
/// "original position" to remember is a screen-space center (from the entity's own
/// `UiGlobalTransform`) rather than a 3D `Transform`.
fn on_placed2d_drag_start(
    trigger: On<Pointer<DragStart>>,
    units: Query<(&UiGlobalTransform, &Placed2DUnit)>,
    mut commands: Commands,
    assets: Res<SharedAssets>,
    mut session: ResMut<DragSession>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    let Ok((transform, &unit)) = units.get(trigger.entity) else {
        return;
    };
    let screen_pos = transform.translation;

    session.team = unit.team;
    session.shape = unit.shape;
    session.origin = DragOrigin::Placed2D;
    session.original_screen_pos = screen_pos;
    session.moving = Some(trigger.entity);

    // Swap to the translucent team-colored preview visuals; shape/size/border stay the same
    // (they don't change), only the coloring does, so the entity/observers keep receiving
    // Drag/DragEnd events for the rest of the gesture exactly like the 3D-unit case.
    let visual = icon_visual(unit.shape, ICON_SIZE, assets.ghost_ui_color(unit.team));
    commands.entity(trigger.entity).insert((BackgroundColor(visual.background), visual.border_color));

    session.origin_marker = Some(spawn_origin_marker(&mut commands, &assets, unit.shape, screen_pos, None));
}

/// While the moving entity (destination preview) is being dragged: a plain 2D preview
/// following the cursor over either side panel, or a 3D preview sliding along the ground
/// (clamped to the dragging team's half) once the cursor crosses into the middle zone. Shared
/// by both palette-started and unit-started drags via `DragSession`. For a brand-new placement
/// (palette origin), the 3D preview also hides itself over an already-occupied spot.
fn on_active_drag(
    drag: On<Pointer<Drag>>,
    mut commands: Commands,
    // Split into a `ParamSet`: `p0` needs `&mut Transform` on the moving entity, `p1` needs a
    // read-only `(&Transform, &PlacedUnit)` over every placed unit for spawn-position
    // resolution. Both touch `Transform`, so they can't be two plain `Query` params in the same
    // system -- but since we only ever use one at a time (never both borrowed simultaneously),
    // a `ParamSet` is safe.
    mut queries: ParamSet<(
        Query<(Option<&mut Node>, Option<&mut Transform>, Option<&mut Visibility>)>,
        Query<(&Transform, &PlacedUnit)>,
    )>,
    windows: Query<&Window>,
    world_camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    assets: Res<SharedAssets>,
    session: Res<DragSession>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    // Pointer<Drag> keeps targeting whatever entity was originally pressed at DragStart -- the
    // palette icon (fixed) for a palette-started drag, or the unit itself for a unit-started
    // drag. Either way, the entity that actually needs to move is `session.moving` (a fresh
    // `Ghost` in the palette case, the same entity as `drag.entity` in the unit case).
    let Some(moving) = session.moving else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.width();
    let pos = drag.pointer_location.position;
    let desired_is_2d = zone_of(pos.x, width) != Zone::World;

    let is_currently_2d = {
        let p0 = queries.p0();
        let Ok((node_opt, _, _)) = p0.get(moving) else {
            return;
        };
        node_opt.is_some()
    };

    if desired_is_2d == is_currently_2d {
        // No zone crossing this event: just follow the pointer.
        if desired_is_2d {
            if let Ok((node_opt, _, _)) = queries.p0().get_mut(moving) {
                if let Some(mut node) = node_opt {
                    node.left = Val::Px(pos.x - ICON_SIZE / 2.0);
                    node.top = Val::Px(pos.y - ICON_SIZE / 2.0);
                }
            }
        } else {
            let (camera, camera_transform) = *world_camera;
            if let Some(point) = point_on_ground(camera, camera_transform, pos) {
                // Resolve the actual spawn-worthy position (immutable borrow) before touching
                // the mutable one below -- repositioning an existing 3D unit never needs to
                // resolve around occupancy (3D -> 3D overlap is allowed), so it just uses the
                // raw clamped point; everything else goes through the shared resolver so the
                // ghost always shows exactly where a drop would actually land.
                let resolved = resolve_drag_position(point.x, point.z, &session, &queries.p1());
                if let Ok((_, transform_opt, visibility_opt)) = queries.p0().get_mut(moving) {
                    if let (Some(mut transform), Some(new_pos)) = (transform_opt, resolved) {
                        transform.translation.x = new_pos.x;
                        transform.translation.z = new_pos.z;
                    }
                    if let Some(mut visibility) = visibility_opt {
                        *visibility = if resolved.is_some() { Visibility::Visible } else { Visibility::Hidden };
                    }
                }
            }
        }
    } else if desired_is_2d {
        // Crossing into a side panel: show the 2D icon preview.
        let visual = icon_visual(session.shape, ICON_SIZE, assets.ghost_ui_color(session.team));
        commands
            .entity(moving)
            .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, Visibility)>()
            .insert((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(pos.x - ICON_SIZE / 2.0),
                    top: Val::Px(pos.y - ICON_SIZE / 2.0),
                    width: visual.width,
                    height: visual.height,
                    border: visual.border,
                    border_radius: visual.border_radius,
                    ..default()
                },
                BackgroundColor(visual.background),
                visual.border_color,
                GlobalZIndex(20),
            ));
    } else {
        // Crossing into the middle zone: show the 3D preview at wherever it would actually
        // spawn (see `resolve_drag_position`).
        let (camera, camera_transform) = *world_camera;
        let raw_point = point_on_ground(camera, camera_transform, pos).unwrap_or(Vec3::ZERO);
        let resolved = resolve_drag_position(raw_point.x, raw_point.z, &session, &queries.p1());
        let fallback_x = clamp_to_team_half(raw_point.x, session.team);
        let (transform, visibility) = match resolved {
            Some(new_pos) => (Transform::from_translation(new_pos), Visibility::Visible),
            None => (Transform::from_xyz(fallback_x, resting_height(session.shape), raw_point.z), Visibility::Hidden),
        };
        commands
            .entity(moving)
            .remove::<(Node, BackgroundColor, BorderColor)>()
            .insert((Mesh3d(assets.mesh(session.shape)), MeshMaterial3d(assets.ghost_material_3d(session.team)), transform, visibility));
    }
}

/// The position a `DragSession`'s ghost preview should show right now, and the same position
/// the actual drop will use (see `on_active_drag_end`) -- always `Some` for `DragOrigin::
/// PlacedUnit` (a straight clamp; 3D -> 3D overlap is allowed and never resolved around), and
/// whatever `resolve_spawn_position` finds (possibly `None`) for a Palette/Placed2D-origin drag
/// entering the world for the first time.
fn resolve_drag_position(raw_x: f32, raw_z: f32, session: &DragSession, units: &Query<(&Transform, &PlacedUnit)>) -> Option<Vec3> {
    if session.origin == DragOrigin::PlacedUnit {
        Some(Vec3::new(clamp_to_team_half(raw_x, session.team), resting_height(session.shape), raw_z))
    } else {
        resolve_spawn_position(raw_x, raw_z, session.team, session.shape, units)
    }
}

/// Resets `entity` to a solid, resting 2D unit at `screen_pos`, in whatever state it was in
/// before (2D or having briefly crossed into 3D during this same gesture). Used both when a
/// Placed2D drag gets rejected (dropped on an occupied 3D spot) and when it's cancelled via
/// right-click -- in both cases the unit already existed, so "cancel" means restore, not
/// discard.
fn restore_placed2d(commands: &mut Commands, assets: &SharedAssets, shape: ShapeKind, team: Team, screen_pos: Vec2, entity: Entity) {
    let visual = icon_visual(shape, ICON_SIZE, assets.solid_ui_color(team));
    commands.entity(entity).remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, Visibility)>().insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(screen_pos.x - ICON_SIZE / 2.0),
            top: Val::Px(screen_pos.y - ICON_SIZE / 2.0),
            width: visual.width,
            height: visual.height,
            border: visual.border,
            border_radius: visual.border_radius,
            ..default()
        },
        BackgroundColor(visual.background),
        visual.border_color,
        GlobalZIndex(10),
        Pickable::default(),
    ));
}

/// Finalizes a drag (shared by all three origins: `Palette`, `PlacedUnit`, `Placed2D`).
///
/// - Dropping over the middle zone places/repositions the unit there, using the same
///   `resolve_drag_position` the ghost preview just showed. For a Palette/Placed2D origin
///   entering the world, a `None` result (no free spot found) is treated as a cancelled drop --
///   discarded for Palette, restored to its original 2D spot for Placed2D. Repositioning an
///   existing 3D unit always resolves (3D -> 3D overlap is allowed).
/// - Dropping over either side panel finalizes (or repositions) the unit as a persistent 2D
///   unit sitting exactly where it was dropped, stripping any combat/movement state -- this is
///   what "3D -> 2D, converting it back" actually leaves behind, and what makes 2D -> 2D
///   dragging (same panel, or across to the other one) work.
///
/// Stats: a brand-new unit (`is_first_placement`) rolls its own randomly jittered `Stats` (see
/// `random_stats`) exactly once, here; anything already placed keeps whatever `Stats` it already
/// has, fetched via `existing_stats` -- repositioning or converting a unit never re-rolls it.
fn on_active_drag_end(
    trigger: On<Pointer<DragEnd>>,
    mut commands: Commands,
    // Whether `moving` is already a placed unit of either kind -- (is-3D, is-2D) -- used to
    // decide whether this is its first-ever placement (attach permanent data/observers) or a
    // reposition/conversion of something that already existed (don't re-attach).
    already_placed: Query<(Has<PlacedUnit>, Has<Placed2DUnit>)>,
    existing_stats: Query<&Stats>,
    placed_units: Query<(&Transform, &PlacedUnit)>,
    windows: Query<&Window>,
    world_camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    assets: Res<SharedAssets>,
    mut session: ResMut<DragSession>,
    mut rng: ResMut<RngState>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    // As in `on_active_drag`: the event targets the icon (palette drags) or the unit itself
    // (unit drags), but the entity to actually finalize/discard is always `session.moving`.
    let Some(moving) = session.moving.take() else {
        return;
    };
    if let Some(marker) = session.origin_marker.take() {
        commands.entity(marker).despawn();
    }

    let Ok(window) = windows.single() else {
        commands.entity(moving).despawn();
        return;
    };
    let width = window.width();
    let pos = trigger.pointer_location.position;

    if zone_of(pos.x, width) == Zone::World {
        let (camera, camera_transform) = *world_camera;
        let raw_point = point_on_ground(camera, camera_transform, pos).unwrap_or(Vec3::ZERO);

        // Same resolution the ghost preview just showed -- see `resolve_drag_position`. `None`
        // means no free spot was found reasonably close (only possible for a Palette/Placed2D
        // origin; PlacedUnit repositioning always resolves), so the drop is refused exactly
        // like a cancelled one: nothing to restore for a palette-started drag, restore a
        // Placed2D-started one to its original 2D spot.
        let Some(spawn_pos) = resolve_drag_position(raw_point.x, raw_point.z, &session, &placed_units) else {
            if session.origin == DragOrigin::Placed2D {
                restore_placed2d(&mut commands, &assets, session.shape, session.team, session.original_screen_pos, moving);
            } else {
                commands.entity(moving).despawn();
            }
            return;
        };

        let (has_3d, has_2d) = already_placed.get(moving).unwrap_or((false, false));
        let is_first_placement = !has_3d && !has_2d;
        let stats = if is_first_placement {
            random_stats(session.shape, &mut rng)
        } else {
            existing_stats.get(moving).copied().unwrap_or_else(|_| Stats::for_shape(session.shape))
        };

        let mut unit = commands.entity(moving);
        unit.remove::<(Node, BackgroundColor, BorderColor, Ghost, Placed2DUnit, Seeking, Attacking)>().insert((
            Mesh3d(assets.mesh(session.shape)),
            MeshMaterial3d(assets.solid_material(session.team)),
            Transform::from_translation(spawn_pos),
            Visibility::Visible,
            Pickable::default(),
        ));

        if session.origin == DragOrigin::PlacedUnit {
            // Pure repositioning: this unit already existed on the battlefield, so its current
            // `Health`/`CombatRanges` carry over untouched -- only its behavior resets, since
            // whatever it was doing before being picked up no longer applies from its new spot.
            unit.insert(Advancing);
        } else {
            // Entering (or re-entering, from a 2D placement) the battlefield: always full
            // strength (relative to its own -- possibly jittered -- max hp), starting out
            // marching.
            unit.insert((Health(stats.hp as f32), combat_ranges(session.shape), Advancing));
        }

        if is_first_placement {
            // First time this entity becomes a real unit: attach its permanent data (including
            // its freshly rolled `stats`), plus the observers it'll need for every future drag
            // of its own. Both DragStart observers (2D and 3D) are attached unconditionally --
            // each only acts when its own marker component is present, so whichever form isn't
            // currently active just makes that one a harmless no-op instead of needing to track
            // "which observers are already attached" separately.
            unit.insert((PlacedUnit { team: session.team, shape: session.shape }, stats))
                .observe(on_unit_drag_start)
                .observe(on_placed2d_drag_start)
                .observe(on_active_drag)
                .observe(on_active_drag_end);
        } else {
            unit.insert(PlacedUnit { team: session.team, shape: session.shape });
        }
    } else {
        // Dropped over a side panel: finalize (or reposition) it as a persistent 2D unit,
        // sitting exactly where it was dropped -- this is what makes "3D -> 2D, converting it
        // back" and repositioning within/between the two panels actually leave something
        // behind, instead of just discarding the drag. Leaving the battlefield strips all
        // combat/movement state; a unit redeployed later starts fresh (see the World-zone
        // branch above).
        let (has_3d, has_2d) = already_placed.get(moving).unwrap_or((false, false));
        let is_first_placement = !has_3d && !has_2d;

        let visual = icon_visual(session.shape, ICON_SIZE, assets.solid_ui_color(session.team));
        let mut unit = commands.entity(moving);
        unit.remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>, Transform, Visibility, Ghost, PlacedUnit)>()
            .remove::<(Health, CombatRanges, Advancing, Seeking, Attacking)>()
            .insert((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(pos.x - ICON_SIZE / 2.0),
                    top: Val::Px(pos.y - ICON_SIZE / 2.0),
                    width: visual.width,
                    height: visual.height,
                    border: visual.border,
                    border_radius: visual.border_radius,
                    ..default()
                },
                BackgroundColor(visual.background),
                visual.border_color,
                GlobalZIndex(10),
                Pickable::default(),
            ));

        if is_first_placement {
            let stats = random_stats(session.shape, &mut rng);
            unit.insert((Placed2DUnit { team: session.team, shape: session.shape }, stats))
                .observe(on_unit_drag_start)
                .observe(on_placed2d_drag_start)
                .observe(on_active_drag)
                .observe(on_active_drag_end);
        } else {
            unit.insert(Placed2DUnit { team: session.team, shape: session.shape });
        }
    }
}

/// Right-clicking at any point during an active drag cancels it: the destination preview and
/// origin marker are removed, and if the drag started from an already-placed unit, that unit
/// is restored to its exact original transform and material.
fn cancel_drag_on_right_click(
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    assets: Res<SharedAssets>,
    mut session: ResMut<DragSession>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    if let Some(marker) = session.origin_marker.take() {
        commands.entity(marker).despawn();
    }
    let Some(moving) = session.moving.take() else {
        return;
    };

    match session.origin {
        DragOrigin::Palette => {
            // Nothing existed before this drag -- just discard the ghost.
            commands.entity(moving).despawn();
        }
        DragOrigin::PlacedUnit => {
            // Restore the exact original environment and position: back to a solid 3D mesh at
            // its original transform. `PlacedUnit`/`Stats`/`Health`/`CombatRanges`/its drag
            // observers were never removed during the drag, only its visual/transform
            // components and its behavior state (stripped at pickup, see `on_unit_drag_start`)
            // -- so it resumes marching from its restored spot, same as any other reposition.
            commands
                .entity(moving)
                .remove::<(Node, BackgroundColor, BorderColor)>()
                .insert((
                    Mesh3d(assets.mesh(session.shape)),
                    MeshMaterial3d(assets.solid_material(session.team)),
                    session.original_transform,
                    Visibility::Visible,
                    Pickable::default(),
                    Advancing,
                ));
        }
        DragOrigin::Placed2D => {
            // Same idea, but restoring to a solid 2D icon at its original screen position.
            restore_placed2d(&mut commands, &assets, session.shape, session.team, session.original_screen_pos, moving);
        }
    }
}