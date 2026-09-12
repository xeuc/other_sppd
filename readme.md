# 3D Shapes Placement Demo (Bevy 0.19.1)

## Run it

```
cargo run --release
```

## How the spec maps to the code

### 1. Layout & coordinates HUD

Three vertical zones: left 20% (Team Blue panel), middle 60% (3D world), right 20% (Team Red
panel). `WorldCamera` gets a real `Camera.viewport` locked to the middle 60% every frame
(`sync_world_camera_viewport`); it's not a full-window camera hidden behind UI. All picking and
placement math goes through `Camera::viewport_to_world`, which already accounts for that
viewport offset.

A HUD in the top-right corner of the 3D area (`update_coords_display`) shows the live 3D ground
coordinates under the cursor, refreshed every frame -- not just on click.

### 2. Teams & shapes

Each side panel lists all three shapes as permanent "spawn button" icons (dragging one never
removes it from the panel):

| Icon | Role      | Mesh        | Stats (HP / ATK / SPD) |
|------|-----------|-------------|-------------------------|
| ●    | Tank      | Sphere      | +++ / ++ / +            |
| ■    | Fighter   | Cube        | ++ / + / +++            |
| ▲    | Assassin  | Tetrahedron | + / +++ / ++            |

`Stats::for_shape` encodes `+ / ++ / +++` as `10 / 20 / 30` and is attached to every placed
unit; nothing reads it yet (reserved for a future combat pass). Left panel = Team Blue, right
panel = Team Red; a unit's color comes from the palette icon (or unit) it was dragged from and
never changes.

### 3. Drag and drop

Two kinds of drag are supported, sharing one `DragSession` resource and the same drag-move /
drag-end logic:

- **Palette → world (2D → 3D):** dragging a sidebar icon spawns a ghost that becomes a
  translucent 3D mesh once it crosses into the middle zone, and is promoted into a permanent,
  opaque unit on a valid drop.
- **Placed unit → anywhere (3D → 3D or 3D → 2D):** every placed unit is itself draggable
  (`on_unit_drag_start`). Dropping it back in the world repositions it (overlaps with other
  units are allowed -- no physics/collision resolution, per spec). Dropping it on either side
  panel removes it (3D → 2D "return to the pool").

Dragging between the two side panels is unrestricted, 2D shapes may overlap freely, and
crossing the 2D/3D boundary swaps `Node` (UI) for `Mesh3d`/`Transform` (world) and back, since
neither the ghost nor a placed unit is ever parented to anything.

### 4. Ghosts

While a drag is active there are two preview entities:

- A **destination preview** that follows the cursor -- a 2D icon over a side panel, or a
  translucent 3D mesh over the world -- always tinted in a lighter, translucent version of the
  *dragging team's* color.
- A static **origin marker** that stays at the drag's exact starting position for the whole
  gesture, always translucent blue regardless of team, so the start and end of the drag are
  visually distinct. Both are removed on a successful drop or a cancellation.

### 5. Invalid 2D → 3D placement

`is_occupied` runs a simple distance check against every placed unit. It's only consulted for a
**brand-new** placement (palette-started drag): while previewing, an occupied target hides the
destination preview instead of showing it as valid; on drop, an occupied target is treated
exactly like a cancelled drop (nothing is created). Repositioning an existing unit (3D → 3D)
never runs this check, so those may overlap freely.

### 6. Right-click cancellation

`cancel_drag_on_right_click` always removes both preview entities. If the drag started from the
palette, nothing existed before it, so that's the whole story. If it started from an
already-placed unit, that unit is restored to its exact original `Transform` and solid
material -- its `PlacedUnit`/`Stats`/drag observer were never touched during the drag, only its
visual/transform components were swapped for previews, so restoring is just re-inserting them.

### 7. 3D team boundaries

The ground is conceptually split at `x = 0`. `clamp_to_team_half` enforces the one-way rule:
Blue can't push past `x = -PLACEMENT_HALF_SIZE`, Red can't push past `x = +PLACEMENT_HALF_SIZE`.
This clamp only touches the X coordinate; Z always follows the cursor freely, so a clamped
preview or unit slides up and down along the centerline instead of getting stuck at one point.
It applies equally to brand-new placements and to repositioning an existing unit.

### 8. Scope

No combat logic, no physics/collision resolution. `Stats` exists as a component but is
otherwise inert, exactly as before.

## Architecture notes

- `DragSession` is the single source of truth for the active drag: which entity is moving,
  which entity is the static origin marker, which team/shape it is, its `DragOrigin`
  (`Palette` or `PlacedUnit`), and (for `PlacedUnit`) the transform to restore on cancel.
- A palette-started drag spawns a fresh `Ghost` entity that gets promoted into a real
  `PlacedUnit` (with its own `on_unit_drag_start` observer attached, once, at that moment) on a
  successful drop -- it's the *same* entity throughout, never despawned and respawned.
- A unit-started drag reuses the placed unit's own entity as the moving/destination preview
  (swapping its material to a translucent one at drag start, and back at drag end/cancel), so
  its existing drag observer keeps receiving events for the rest of the gesture.
- `on_active_drag` and `on_active_drag_end` are shared by both flows; only `on_palette_drag_start`
  vs. `on_unit_drag_start` differ, plus a couple of `DragOrigin`-gated branches (the occupancy
  check, and what a cancellation restores).

## A note on verification

I checked every non-obvious API call used here against the official Bevy 0.19 documentation and
examples (the `On<Pointer<E>>` observer fields, `Camera::viewport_to_world` with a non-zero-
offset viewport, `BorderRadius`, `GlobalZIndex`, `IsDefaultUiCamera`, the `Alpha` trait,
`TextFont`, the `px()`/`percent()` helpers, `UiTransform` vs `Transform`, `ParamSet`, etc.). Two
spots are closer to guesses and worth double-checking once it runs:

- `resting_height` for the Triangle/Tetrahedron shape (`0.6`) is an approximation -- Bevy's
  default `Tetrahedron` doesn't expose a documented size to compute this from exactly, so the
  pyramid may sit slightly above or below the ground until you nudge that constant.
- `OCCUPANCY_RADIUS` (`0.8`) is a placement-blocking heuristic, not derived from each shape's
  actual footprint -- nudge it if placements feel too permissive or too strict.

I still wasn't able to actually compile this: Bevy 0.19.1 requires a newer Rust toolchain than
is available in the sandbox I write this in, with no way to install one. Please run
`cargo check` before relying on it -- in particular, double check the `ParamSet` usage in
`on_active_drag` (needed because it touches `Transform` both mutably, on the dragged entity,
and immutably, over all placed units, for the occupancy check) compiles and behaves as
commented.