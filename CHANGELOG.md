# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
