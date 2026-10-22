# Feature adoption plan — from the competitor survey

Date: 2026-10-22 · Source: competitor research on 14+ Windows room/home
design products (Room Arranger, Sweet Home 3D, Live Home 3D, Planner 5D,
SketchUp, Chief Architect Home Designer, Punch!, Virtual Architect,
RoomSketcher, Floorplanner, Homestyler, Cedreo, DreamPlan, HomeByMe,
magicplan), from official product/help/pricing pages, 2026-10-22.
SEAN's direction: "adopt as many features as possible."

**Rules that still bind** (SEAN's recorded decisions — adopting a
feature never silently overrides them): no prices or cost estimators
anywhere; generic original catalog only; core stays fully offline.
Features that collide with those rules are listed at the bottom as
**Needs SEAN's call**, not adopted.

Status key: ✅ in v0.1 · **Adopt → M2/M3/M4** = added to that milestone
in the product plan · Later = v2+ backlog.

## Table stakes (nearly every tool has them — we must match)

| Feature | Where seen | RoomForge |
|---|---|---|
| Typed wall dimensions, wall thickness + height | all desktop tools | ✅ dims/height; thickness ✅ v0.2.0 |
| Metric + imperial units | Punch!, RoomSketcher, all | ✅ v0.2.0 (display + typed input; exports metric for now) |
| Doors/windows auto-cut into walls, swing shown | Sweet Home 3D, all | ✅ (visual inset/swing; true cut with wall segments → M2) |
| Drag-and-drop catalog organised by room | all | ✅ |
| Simultaneous 2D + 3D of the same design | all | ✅ |
| Trace a plan/blueprint image, calibrated by a known length | Room Arranger, Sweet Home 3D, RoomSketcher, DreamPlan | ✅ v0.3.0 (embedded image, two-point calibration) |
| Room/wall area readouts | Room Arranger (→Excel), RoomSketcher | ✅ room area; per-wall + export Adopt → M2 |
| Dimension lines on the plan | Sweet Home 3D, RoomSketcher | ✅ v0.2.0 |
| Text annotations on plan | Room Arranger, Sweet Home 3D | ✅ v0.2.0 (notes pinned to the plan) |
| Starter templates / sample rooms | all | ✅ starter room; template set Adopt → M3 |
| Local save, undo/redo | all (autosave rarely documented; Live Home 3D documents it) | ✅ incl. autosave + recovery (already ahead) |
| Plan image export + print to scale (1:50/1:100, tiled pages) | Room Arranger, Sweet Home 3D, RoomSketcher Pro | ✅ SVG/print sheet; true-scale 1:50/1:100 SVG ✅ v0.3.0; page tiling later |
| Walkthrough / first-person view | all desktop tools | Adopt → M4 (was v2; pulled forward) |
| Stairs + multiple floors | all except minimal tools | Adopt → M4 (format v2 already designed for it) |

## Common differentiators (adopt the ones that fit an offline planner)

| Feature | Where seen | RoomForge |
|---|---|---|
| Import 3D models (OBJ/glTF) for custom furniture | Live Home 3D (OBJ/glTF/FBX…), Sweet Home 3D (OBJ/DAE/3DS), Planner 5D Pro | **Adopt → M5** (GLB first, per-file license note; already in plan) |
| Custom object builder (build furniture from boxes/parts in-app) | Room Arranger standout | **Adopt → M3** — simple box-composed custom items, stored in the project |
| Custom textures from image files; PBR material sets | Chief Architect, Punch!, Live Home 3D | Adopt → M3 (with the CC0 pack; provenance manifest) |
| Curved walls | Punch!, Chief Architect | Later (after wall segments prove out) |
| Offline photoreal rendering (path-traced stills) | Punch! LuxCore 8K, Chief Architect ray tracing | Adopt → M3 as *offline still render*; our renderer grows shadows/soft light first |
| Lighting design: fixtures, switches, time-of-day sun | Chief Architect (lights↔switches), Virtual Architect (geo sun) | Adopt → M4 (fixtures + day/evening already planned; switch linking later) |
| PDF / DXF export of the plan | Floorplanner (paid), RoomSketcher Pro, Homestyler (DWG) | PDF print Adopt → M2 (via print sheet); DXF Later |
| Dollhouse view, 360° panorama export, camera-path video | Live Home 3D, Planner 5D, Sweet Home 3D (video) | Dollhouse = our orbit view ✅-ish; 360/video Later |
| Electrical/plumbing symbol sets on the plan | Cedreo, Chief Architect, Homestyler | ✅ v0.3.0 — seven service kinds, one shared symbol table |
| Materials list (quantities, no money) | Chief Architect, Punch! | **Adopt → M3** — quantities/areas only. Cost versions are in Conflicts. |
| Sun-study / shadow animation | Chief Architect class | Later |
| Human-height measurement checks, door-clearance rules | scattered | Door-swing clearance ✅ v0.2.0; human-height checks later |

## Rare / advanced (backlog, not scheduled)

- In-app plug-in/scripting system (Sweet Home 3D Java plug-ins) — Later,
  only if a real extension need appears.
- Automatic roofs, foundations, framing, HVAC plan generation
  (Chief Architect, Punch!) — Later; roofs for whole-home view maybe M4+.
- Elevation/section views generated from the model (Cedreo, Live Home
  3D Pro elevation) — Later.
- Plant/outdoor/terrain (Punch!, Virtual Architect, Live Home 3D) —
  Later; outdoor rooms are a different product surface.
- VR/AR viewing — Later, needs the wgpu/three-d renderer step first.

## Our differentiators (keep and sharpen — the survey's paradox)

The survey's sharpest finding: across 14 products, **unmetered offline
rendering with no watermark, no credits and no account** is rare enough
to be a headline. RoomForge already has: fully offline everything, no
account, no credits/watermarks, autosave + crash recovery (documented
by only one competitor), an open inspectable project file, and MIT
open source. These stay front and centre in the README.

## Needs SEAN's call (NOT adopted — they collide with his decisions)

1. **Cost estimators / priced materials lists** (Punch! estimator,
   Chief Architect, Planner 5D Budget Widget): collides with SEAN's
   2026-10-22 "no prices" decision. Quantities-only lists are adopted
   above; money stays out unless SEAN reverses it.
2. **Cabinet/kitchen configurators** (Chief Architect cabinets,
   Planner 5D custom furniture): collides with the configurator
   exclusion. The box-composed custom-object builder (adopted) covers
   most of the user need without a configurator.
3. **Photo/LiDAR/AI scan-to-plan** (Room Arranger LiDAR via iOS,
   magicplan AR/LiDAR, Planner 5D AI plan recognition, Homestyler
   RoomScan): collides with the photo/scan exclusion, and most of it
   needs a phone/cloud anyway. Manual blueprint **tracing** (adopted)
   is the offline, in-scope version of the same job.
