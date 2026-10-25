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
| Starter templates / sample rooms | all | ✅ starter room; template set ✅ v0.4.0 (Living room / Bedroom / Home office) |
| Local save, undo/redo | all (autosave rarely documented; Live Home 3D documents it) | ✅ incl. autosave + recovery (already ahead) |
| Plan image export + print to scale (1:50/1:100, tiled pages) | Room Arranger, Sweet Home 3D, RoomSketcher Pro | ✅ SVG/print sheet; true-scale 1:50/1:100 SVG ✅ v0.3.0; page tiling later |
| Walkthrough / first-person view | all desktop tools | Adopt → M4 (was v2; pulled forward) |
| Stairs + multiple floors | all except minimal tools | Adopt → M4 (format v2 already designed for it) |

## Common differentiators (adopt the ones that fit an offline planner)

| Feature | Where seen | RoomForge |
|---|---|---|
| Import 3D models (OBJ/glTF) for custom furniture | Live Home 3D (OBJ/glTF/FBX…), Sweet Home 3D (OBJ/DAE/3DS), Planner 5D Pro | **Adopt → M5** (GLB first, per-file license note; already in plan) |
| Custom object builder (build furniture from boxes/parts in-app) | Room Arranger standout | ✅ v0.4.0 — custom-sized box items with name/group/colour, stored in the project (multi-part composition later) |
| Custom textures from image files; PBR material sets | Chief Architect, Punch!, Live Home 3D | Partial ✅ v0.6.0 — procedural materials (8 flooring presets: plank/tile/joint patterns drawn from the model, no texture files to lose). User image textures still Later. |
| Curved walls | Punch!, Chief Architect | Later (after wall segments prove out) |
| Offline photoreal rendering (path-traced stills) | Punch! LuxCore 8K, Chief Architect ray tracing | Partial ✅ v0.6.0 — offline PNG snapshot of the 3D view via a software rasterizer (flat-shaded, not photoreal; shadows/soft light Later) |
| Lighting design: fixtures, switches, time-of-day sun | Chief Architect (lights↔switches), Virtual Architect (geo sun) | Adopt → M4 (fixtures + day/evening already planned; switch linking later) |
| PDF / DXF export of the plan | Floorplanner (paid), RoomSketcher Pro, Homestyler (DWG) | PDF print Adopt → M2 (via print sheet); DXF Later |
| Dollhouse view, 360° panorama export, camera-path video | Live Home 3D, Planner 5D, Sweet Home 3D (video) | Dollhouse = our orbit view ✅-ish; 360/video Later |
| Electrical/plumbing symbol sets on the plan | Cedreo, Chief Architect, Homestyler | ✅ v0.3.0 — seven service kinds, one shared symbol table |
| Materials list (quantities, no money) | Chief Architect, Punch! | ✅ v0.4.0 — floor/ceiling/wall areas + skirting/perimeter, in list view and print sheet. Cost versions are in Conflicts. |
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

## SketchUp Pro deep-dive (2026-10-24)

SEAN directed a focused pass on SketchUp Pro (Trimble's plans page +
help material, 2026-10-24) as further inspiration for RoomForge.
SketchUp itself was one of the 15 products in the original survey;
this is the deeper, feature-by-feature classification. Same rules
bind: no prices, generic/user-owned content only, fully offline.

**Adopted now (v1.3.0, the presentation pack):**

| SketchUp Pro | RoomForge form |
|---|---|
| Scenes — saved views capturing camera, style and section state | **Saved views** stored in the plan: orbit angle/zoom, lighting scene, render style and wall cut, recalled in one click |
| Styles — one-click visual styles (incl. monochrome/clay looks) | **Clay render style** alongside the normal coloured style, in 3D, walk and snapshots |
| Section planes / section cuts | **Wall cutaway (Full / Half / Low)** in the 3D views — the dollhouse cut, as a height rather than a freeform plane (RoomForge rooms are single-storey shells) |

**Already had (SketchUp equivalents):** inference snapping →
alignment guides (v1.0.0); Tape Measure → dimension lines, wall
clearances, typed lengths; Components → custom items + imported
models (placed RoomForge items hold their own size rather than
following a definition — a deliberate difference); Materials →
procedural finishes (v0.6.0), user image textures already on the
roadmap; LayOut → print sheet + true-scale SVG (page tiling still
roadmapped); Outliner → per-room item list (v1.1.0).

**Later:** drawing rooms as free polygons (the floor-plate side of
Push/Pull — needs general polygon outlines beyond rectilinear
notches); sun-position shadow studies (their shadows/PreDesign);
fog; camera-path video; a tags/layers visibility panel; titled
print layouts with page tiling.

**Not adopted — conflicts with SEAN's standing rules:** 3D
Warehouse (third-party model library; generic catalog + the user's
own imports cover the need), Trimble Connect cloud storage and
collaboration (offline rule), Extension Warehouse / plug-in API
(scope), SketchUp AI generation and AI render (cloud AI; offline
rule), Scan Essentials point clouds and Match Photo (scan/photo
modelling was excluded 2026-10-22), and general-purpose CAD
modelling (solid booleans, Follow-Me extrusions — geometry
authoring, not room planning).

## Madera deep-dive (2026-10-25)

SEAN directed a pass on Madera (madera.app — "Simple furniture
builder", observed 2026-10-25). What could be observed without an
account: a template-first start (Closet, Double Closet, Chest of
drawers, Side Table), a project viewer with an **assembly preview**
(Normal / **Open doors and drawers** / **Space modules — pull the
panels apart**) and **canvas render modes** (a lit render, and a
**Technical drawing — outlines only**). The builder itself sits
behind an email sign-in and was not entered; nothing behind that
wall is claimed here.

**Adopted now (v1.4.0, the assembly pack) — its presentation ideas,
translated from one piece of furniture to a whole room:**

| Madera | RoomForge form |
|---|---|
| Open doors and drawers preview | Storage, kitchen and bathroom pieces gain real front panels (wardrobe doors, chest drawers, oven door…) and an **Open doors & drawers** toggle in the 3D views swings/slides them open |
| Space modules — pull panels apart | **Exploded view** slider: furniture pieces drift apart from the room's centre so the layout reads at a glance |
| Technical drawing — outlines only | An **Outline** render style beside Coloured/Clay: pale paper surfaces with dark edges, in 3D and snapshots |

**Already had:** template cards as the starting point (the v1.0.0
template gallery); Remix ≈ duplicate/rename flows.

**Needs SEAN's call — NOT adopted (at the time):** Madera's core
is designing one piece of furniture from modules and panels — a
**modular furniture configurator**, the category SEAN excluded on
2026-10-22 (see AGENTS.md hard rules).

**UPDATE 2026-10-25 — SEAN made the call:** he reversed the
exclusion ("add the furniture configurator"). v1.5.0 adds the
Furniture Designer: parametric pieces from panels with finishes
and options, a dimensions-and-quantities cut list (no prices —
that rule stands), and placement into rooms as real parts. The
sign-in/cloud model remains NOT adopted (offline rule), and
selling-price calculators remain out (no-prices rule).
