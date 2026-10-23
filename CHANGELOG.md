# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
