# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0] - 2026-10-24

The presentation release: three ideas borrowed from professional
3D modellers, translated for a room planner.

### Added

- **Saved views** (file format v5): "Saved views…" in the 3D view
  stores the whole presentation — camera angle and zoom, lighting
  scene, render style and wall cut — under a name, inside the
  plan. Clicking a saved view restores it all at once; views can
  be deleted, and they travel when a plan is shared.
- **Clay render style**: a Style row in the 3D view (and in the
  walkthrough) switches between the room's real colours and a
  warm-grey "clay" look that flattens every surface by brightness
  — for judging shapes and layout without colour getting in the
  way. PNG snapshots use whichever style is showing.
- **Wall cutaway**: a Walls row (Full / Half / Low) caps how much
  wall the 3D views draw, so you can see into the room like a
  section cut. Window glass clips at the cut, windows above it
  hide, and snapshots honour the cut too. Rendering only — the
  plan itself is untouched, and the walkthrough always keeps
  full-height walls (you are standing inside).

## [1.2.0] - 2026-10-23

### Added

- **Import your own 3D models** (file format v4): "Your 3D
  models" in the catalogue imports `.glb` (binary glTF) and
  `.obj` files up to 10 MB. An imported model places like any
  furniture — footprint, resizing, lists, warnings — and renders
  with its real geometry and colours in the 3D view, the
  walkthrough and both snapshot kinds. The 2D plan shows its
  footprint. Models are stored inside the plan file, so plans
  stay portable; deleting a model definition makes its placed
  copies fall back to simple boxes. Supported subset (by design):
  static meshes with per-part base colours — no animation, skins
  or textures; very detailed models (over ~500k triangles) are
  declined with an explanation.

## [1.1.0] - 2026-10-23

### Added

- **Multi-room plans** (file format v3; older files open as
  single-room plans): a plan can now hold a whole home. A Rooms
  strip under the top bar switches rooms; "+ Add room" starts a
  new room from any template; rooms can be duplicated and deleted
  from the Room panel (the last room is always kept). Each room
  keeps its own dimensions, flooring, walls, fittings, furniture,
  notes and tracing image.
- **Doors can say where they lead**: a door's editor gains a
  "Leads to" picker naming another room; the label shows in the
  item list and the print sheet ("Door — East wall … → Study").
- **Whole-home reporting**: with several rooms, the item list
  shows a section per room plus whole-home materials totals, the
  print sheet prints every room with a totals page section, and
  the status bar counts furniture across the plan.

## [1.0.1] - 2026-10-23

### Fixed

- **Imperial units are now honoured everywhere they're shown**: the
  item list's doors/windows/services lines, the print sheet (header,
  item sizes, fittings, materials) and the SVG plan labels all use
  feet/inches and ft² when Imperial is selected, instead of silently
  staying metric.
- **The Bedroom template no longer starts with a warning**: its
  floor lamp stood on the double bed's corner. The bed, bedside
  table and lamp were nudged apart, and a regression test now
  proves all three starter templates load warning-free.

## [1.0.0] - 2026-10-23

The feel release: the same planner, finished to feel like the
polished online tools — direct manipulation, magnetic alignment,
and pictures that look like a room. (Still offline, still no
prices, still a generic catalog — by design.)

### Added

- **Drag furniture straight in**: catalog rows can be dragged onto
  the plan and dropped where you want them (click-to-place still
  works exactly as before).
- **Magnetic alignment while dragging**: pieces snap to other
  pieces' edges and centres, to the walls and to the room's centre
  lines, with red guide lines showing what caught — and a live
  readout of the gap to each wall as you drag.
- **Skirting boards and soft shadows** in the 3D view, the
  walkthrough and PNG snapshots alike: little boards along every
  wall base and a soft shadow under each piece of furniture.
- **Catalog cards**: each piece is shown as a small top-down
  drawing of itself (a sofa with arms, a bed with its pillow band,
  a wardrobe with doors) with its size in your units.
- **Template gallery** (New → Template gallery…): the starter
  rooms as picture cards with live mini-plans, pickable in one
  click.
- **Walk snapshots**: "Save PNG of this view" in walk mode renders
  a picture from where you're standing.

## [0.9.0] - 2026-10-23

### Added

- **Walk through the room**: a first-person mode inside the 3D
  view. Stand where a person would stand (eye height 1.6 m) and
  walk the floor with W A S D or the arrow keys; Q/E or dragging
  turns your head, Shift hurries, Esc steps back out. Movement has
  real collisions — you slide along walls and furniture (flat rugs
  can be walked over), so the walk is an honest test of walkways
  and clearances. Lighting scenes apply in the walk too. The maths
  (true-perspective camera, near-plane clipping, collision) lives
  in the library with its own tests, like everything else here.

## [0.8.0] - 2026-10-23

### Added

- **Lighting scenes for the 3D view**: Day, Evening and Night
  buttons beside the orbit controls change the whole mood of the
  room — overall brightness, a warm tint at evening, a cool one at
  night, and window glass that goes from sky blue to sunset orange
  to moonlight. PNG snapshots render in whichever scene is showing.
  Scenes are deliberately simple global moods (no per-lamp lighting
  or shadows) — the README says so plainly.

## [0.7.1] - 2026-10-23

### Added

- **An icon of its own**: a top-down floor plan with a red sofa and
  a door swing, in the black/red family colours. It's embedded in
  the .exe file itself (Explorer, taskbar, shortcuts) and used as
  the window icon. Sources live in `assets/` (`icon.png`, `icon.ico`).
- **A banner for the README** in the same style (`docs/banner.png`).

## [0.7.0] - 2026-10-23

A usability release: the same RoomForge, easier to learn and to use,
whether it's your first room plan or your fiftieth.

### Added

- **Built-in Help (F1 or the Help button)**: a plain-language guide
  covering the four steps to a first room, what the few jargon words
  mean ("services", "notch", "tracing image"), every keyboard
  shortcut, and the good-to-know facts (fully offline, autosave and
  recovery, why there are no prices).
- **"Space around it" readout**: selecting a piece of furniture now
  shows the clear gap between it and each wall — the quick way to
  check a walkway or door clearance without measuring by eye.
- **Text size controls** (A− / A+ at the right of the Export row):
  scale everything in the window from 85% to 150%.

### Changed

- Plainer labels throughout: the 3D sliders are now "Turn" and
  "Tilt" (not Yaw/Pitch); an item's position fields read "Centre —
  from left wall" / "Centre — from front wall"; the notch fields
  are "Notch width" and "Notch depth"; the selection panel ends
  with a reminder of its keys.
- The Blueprint tracing controls now live in a collapsed section of
  the Room panel, so the everyday controls are easier to scan.

## [0.6.1] - 2026-10-23

Fixes from the first on-machine run of v0.6.0 (thank you for the
screenshots — they're in the README now).

### Fixed

- 3D view: floor-pattern lines were painted over everything —
  furniture and even the near walls, which made the walls read as
  pale flooring. Pattern lines are now depth-sorted with the scene,
  sitting on the floor and passing under furniture, exactly as in
  PNG snapshots.
- 3D view: the "Save PNG snapshot" button was clipped by the side
  panel on narrower windows; it now has its own row under the
  orbit sliders.
- Warnings: flat items (rugs) no longer raise "overlaps" warnings
  for the furniture standing on them — that's what rugs are for.
- The starter living room's floor lamp no longer grazes the sofa
  (it raised an overlap warning in a brand-new plan).

## [0.6.0] - 2026-10-23

### Added

- **Flooring materials.** Eight procedural finishes — oak, walnut and
  grey-laminate planks (with staggered joints), ceramic and slate
  tiles, beige carpet, polished concrete (with expansion joints), or
  a solid colour of your own. The pattern is drawn from the model, so
  it follows L-shaped outlines and appears consistently in the 2D
  plan, the 3D view, SVG exports and print sheets. No texture files
  are needed, so plans stay self-contained.
- **Paint and floor swatches** in the Room panel for one-click wall
  and floor colours, alongside the full colour pickers.
- **"Save PNG snapshot"** in the 3D view: renders the standard 3D
  view to a 1280×800 PNG picture you can share — rendered by an
  offline software rasterizer, so it works anywhere and matches what
  the plan describes (flat-shaded, not photorealistic).

## [0.5.0] - 2026-10-22

### Added

- **Non-rectangular rooms (file format v2).** Cut a rectangular notch
  out of any corner of the room — one notch makes an L-shape, more
  make U/C shapes — with independent sizes along each wall, and
  restore the plain rectangle at any time. Everything adapts: floor
  area and materials use the true outline, furniture and notes are
  kept inside the real shape, doors/windows/services re-clamp to the
  wall runs that still exist, and the 2D plan, 3D view and SVG export
  all draw the actual outline (including walls on the notch edges).

### Changed

- Project files are now format version 2. Plans saved by any earlier
  release load unchanged (they simply have no outline) and are saved
  back as v2. Older RoomForge versions will refuse v2 files with the
  usual "saved by a newer version" message.

## [0.4.0] - 2026-10-22

The third adoption batch from the competitor feature survey.

### Added

- Custom-object builder: define your own furniture (name, group,
  width × depth × height, colour) under "Your items" in the catalog
  panel, then place it like any catalog piece. Definitions are stored
  in the plan file, so they travel when you share a plan.
- Materials list with quantities only: floor/ceiling area, wall area
  gross and net of doors/windows, skirting length and perimeter —
  shown in the item-list view and on the print sheet. Areas and
  lengths, never money.
- Room templates in the New menu: furnished Living room, Bedroom and
  Home office starting points, alongside the empty room.

## [0.3.0] - 2026-10-22

The second adoption batch from the competitor feature survey.

### Added

- Blueprint tracing: load a scanned or photographed floor plan
  (PNG/JPEG/BMP) and it appears under the 2D drawing, fitted to the
  room. Calibrate its true scale by clicking two points whose real
  distance you know; adjust opacity while you trace. The image is
  embedded in the plan file, so shared plans keep their underlay.
- True-scale print exports: "Plan 1:50" and "Plan 1:100" SVG buttons
  produce files with physical dimensions — print at 100% and the
  drawing is exactly to scale.
- Four more wall service points: light switch, TV point, network
  point and thermostat, each with its own plan symbol in the app and
  in exports (letters/colours now come from one shared table).

### Notes

- Two new dependencies, both permissively licensed and used offline
  only: `image` (decoding plan images) and `base64` (embedding them).

## [0.2.0] - 2026-10-22

The first adoption batch from the competitor feature survey
(`docs/FEATURE_ADOPTION.md`).

### Added

- Imperial units: a Metric/Imperial toggle drives every readout, and
  the room and furniture size fields accept typed lengths in either
  system (5 m, 500 cm, 10' 6", 126"). Exports remain metric in this
  release.
- Wall thickness on the room, drawn to scale on the 2D plan and in the
  SVG export; room dimensions stay clear internal dimensions.
- Dimension lines with end ticks on the 2D plan and the SVG export.
- Text notes pinned to the plan (📝 Add note), numbered on the plan,
  listed in the item-list view and on the print sheet / SVG export.
- Door-swing warnings: furniture standing in an inward door's swing
  area is now flagged, like overlaps and covered radiators.

### Notes

- Plans saved by 0.1.0 open unchanged (new fields default sensibly).

## [0.1.0] - 2026-10-22

### Added

- Room planner for Windows: draw a room to scale with walls, doors,
  windows, electrical sockets, pipes/water points and radiators.
- 2D plan view with drag-and-drop furniture from a generic, original
  catalogue; move, rotate, duplicate and delete, with undo and redo.
- Real-time 3D view of the same room with orbit, pitch and zoom.
- Item list panel listing everything placed in the room.
- Save, open and share plans as versioned `.roomforge` JSON files,
  with autosave and atomic writes.
- Print/export of the plan and of the item list.
- Keyboard shortcuts for nudging, rotating, deleting, undo/redo,
  saving, and switching between the 2D, 3D and item list views.
- Black and red family theme.

### Notes

- There are no prices in RoomForge, by design — see `DECISIONS.md`.
- First build: runtime behaviour on a real Windows machine is not yet
  verified; see the known-limitations notes in the project records.
