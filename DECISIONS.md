# Decisions

A running log of significant decisions for RoomForge: what was decided,
why, and what it means. Newest entries first within each date.

## 2026-10-24

### Presentation state: saved views, clay style, wall cut (v1.3.0, format v5)

- **Decision:** A SketchUp Pro inspiration pass (see
  docs/FEATURE_ADOPTION.md, 2026-10-24 section) adopted its three
  presentation ideas in planner form. Saved views persist camera +
  scene + style + cut in the plan (`SavedView`, format v5) — views
  are plan content, not app preferences, so they travel. The clay
  style is a pure colour mapping (`view3d::style_rgb`) applied at
  every shade call-site in all renderers, never a model change.
  The wall cut is a render-time height cap (`WallCut::cut_height`)
  applied in the orbit view and snapshots only; the walkthrough
  ignores it because the viewer is inside the room.
- **Why:** Presenting a plan (to family, a builder, one's future
  self) is half the product's job; named views make good angles
  repeatable, clay removes colour noise when judging layout, and
  a height cut is the honest equivalent of a section plane for
  single-storey room shells.
- **Consequences:** Glass is clipped at the cut and windows above
  it hidden in cut views — documented behaviour, not a bug. Style
  and cut are live prefs; only saved views persist them.

## 2026-10-23

### Model import: hand-rolled GLB/OBJ subset, prepared unit space (v1.2.0, format v4)

- **Decision:** Import is a small library module (`model3d`) with
  hand-written parsers for the static-mesh subset of GLB and OBJ —
  no new dependencies. Parsers convert to the room's Z-up frame
  and prepare models into a unit space (width 1, centred, grounded);
  the definition stores the source file base64 plus the depth/
  height ratios, and rendering scales onto the placed item's
  footprint (`view3d::mesh_tris`, brightness from face normals,
  double-sided). All three render paths share the branch: model
  items draw triangles, everything else boxes, and a missing or
  unparseable definition falls back to the box.
- **Why:** Furniture models people actually have are static
  meshes; animation/skins/textures would multiply the work for
  little planning value. Storing the source file (not cooked
  geometry) keeps plans honest and re-preparable; parsing is
  cached per definition in the GUI and done per-call in the
  one-off snapshot renderer.
- **Consequences:** OBJ materials (.mtl) and glTF textures are
  ignored — parts render in their base colour (OBJ: a warm grey).
  Non-uniform resizing stretches a model, exactly as it stretches
  its footprint; the properties panel labels model items so the
  behaviour is discoverable.

### Multi-room plans: active room + stored rooms (v1.1.0, format v3)

- **Decision:** A project keeps the ACTIVE room in its original
  fields and stores the other rooms whole in `rooms: Vec<StoredRoom>`;
  switching swaps the two (in place, so tab order is stable). File
  format bumps to v3 — the new fields are serde-defaulted, so v1/v2
  files load as single-room plans and are repaired on load (active
  room id assigned, duplicate room ids reissued, dangling door
  links cleared). `add_room` reissues every inner id from the
  project's counter, because template projects hand out the same
  small ids. Doors gain an optional `links_to` room label.
- **Why:** Every renderer, the walkthrough, collision, materials
  and warnings already work on "the room in the fields" — this
  design adds multi-room without touching any of them, and
  `Project::as_single` lets exports treat each room with the exact
  single-room code paths. A shared-canvas floor plan (rooms side
  by side with shared walls) was the alternative; it would have
  rewritten the geometry core for a benefit most home plans don't
  need — rooms here are designed one at a time, like the online
  planners' room-by-room flow.
- **Consequences:** Rooms don't share walls or a canvas; a door
  "leads to" another room as a label only. Exports of a single
  room's plan SVG/snapshot still operate on the active room; the
  print sheet and item list aggregate the home.

### The v1.0 feel pack: polish inside the standing scope (v1.0.0)

- **Decision:** SEAN asked for the feel of the IKEA online planner.
  Built: drag-from-catalog, magnetic alignment guides (a pure
  `geometry::align_snap`), skirting boards + blob shadows shared
  by all three render paths, catalog glyph cards, a template
  gallery, and walk-view snapshots. NOT built, per SEAN's standing
  2026-10-22 exclusions: real manufacturer products/photos, prices
  or cart totals, product configurators, photo/scan capture.
- **Why:** The remaining gap to the online tools was interaction
  finish, not capability — and all of it was achievable with the
  generic catalog and the no-prices rule intact.
- **Consequences:** Alignment snapping applies on top of (and per
  axis, ahead of) the 5 cm grid snap. Shadows are darkened-floor
  footprints, not shadow mapping — consistent with the flat-shaded
  renderer and honest in the docs.

### Walkthrough: a real perspective camera in the library (v0.9.0)

- **Decision:** The walkthrough gets its own module (`src/walk.rs`)
  rather than stretching the orbit projector: a compass-heading
  `Walker`, a true-perspective transform with pitch, Sutherland–
  Hodgman clipping against a near plane, and a painter's-order
  face list mirroring the orbit scene (floor pattern, ceiling,
  walls, boxes, glass). Movement is axis-separated with a body
  radius, probing the outline polygon and inflated furniture boxes;
  flat items (≤ 5 cm) are walkable. The GUI only forwards keys and
  paints.
- **Why:** The orbit camera is a weak-perspective toy that cannot
  stand inside a room; near-plane clipping is the difference
  between a walkthrough and a mess of smeared polygons at every
  wall. Keeping it in the library keeps it testable, like the rest.
- **Consequences:** Walk snapshots aren't offered yet (the orbit
  snapshot is the shareable picture). Eye height is fixed at 1.6 m
  for now.

### Lighting scenes are global moods (v0.8.0)

- **Decision:** `Scene` (Day/Evening/Night) lives in the library
  (`view3d::Scene` + `shade_scene`) and drives both the live 3D
  view and the snapshot renderer through one code path: an ambient
  multiplier, a per-channel tint, a backdrop colour and a glass
  colour/alpha. No per-lamp lights, shadows or radiosity.
- **Why:** The roadmap promised "day/evening scenes"; a global mood
  is the honest 80% — it answers "how does this room feel at night?"
  without pretending to simulate lighting. Anything richer needs a
  real lighting model, which is out of scope for a flat-shaded
  planner.
- **Consequences:** The 2D plan is a technical drawing and ignores
  scenes entirely. If per-lamp lighting is ever added, it replaces
  `shade_scene`'s tint step, not the scene plumbing.

### Usability pass: plain language first (v0.7.0)

- **Decision:** Treat learnability as a feature: a built-in Help
  window (F1) written in plain language with a small glossary,
  controls labelled by what they do ("Turn"/"Tilt", "Centre — from
  left wall") rather than by technical terms, a "space around it"
  clearance readout for the selected item, and A−/A+ text scaling.
  Advanced capability stays (typed unit expressions, true-scale
  exports, calibration) but is introduced as "pros" notes, never as
  the first thing a beginner meets.
- **Why:** SEAN's direction — the program must be accessible to
  amateurs and professionals alike. The first on-machine run showed
  the features all worked; the remaining gap was discoverability
  and vocabulary.
- **Consequences:** New UI text is held to the same standard: say
  what a control does, pair every colour cue with words, and put a
  one-line explanation wherever a term is unavoidable.

### Procedural materials and a software snapshot renderer

- **Decision:** Floor finishes are procedural (base colour + pattern
  segments generated from the model), not image textures; the 3D
  snapshot export is a small software rasterizer in the library
  (painter's algorithm), not a screenshot of the GUI.
- **Why:** Plans stay self-contained (no texture files to lose or
  license), patterns follow notched outlines exactly, one segment
  generator feeds all four renderers, and the snapshot works
  headless and is unit-testable. User-supplied image textures remain
  a possible later addition.
- **Consequences:** Finishes are flat-shaded, not photoreal; the
  snapshot uses the standard camera rather than the live orbit
  position.

## 2026-10-22

### Format v2: rectilinear outlines via corner notches

- **Decision:** Rooms gain an optional rectilinear outline (format
  v2). The user makes L/U/C shapes by cutting rectangular corner
  notches (sized along each wall) rather than by free wall drawing.
  Openings and services stay attached to the four outer walls by
  offset, clamped to the wall runs that survive the notches.
  Furniture/notes clamp to the true polygon.
- **Why:** It covers the overwhelmingly common real-room shapes with
  a small, fully testable model (polygon area/perimeter/contains,
  run-based clamping) instead of a general wall-segment editor,
  which remains a possible later step. Migration is free: v1 files
  have no outline and load as rectangles.
- **Consequences:** Free-form/angled walls are still unsupported;
  openings cannot sit on notch edges yet; the underlay still maps to
  the bounding box.

### Competitor feature adoption

- **Decision:** Following SEAN's "adopt as many features as possible"
  after a survey of 14+ existing Windows room/home-design products,
  `docs/FEATURE_ADOPTION.md` maps their features onto the milestones:
  table stakes to match (wall thickness, imperial units, dimension
  lines, blueprint tracing, scaled print), differentiators to adopt
  (custom-object builder, GLB/OBJ import, offline photoreal stills,
  quantities-only materials lists), and a rare/advanced backlog.
- **Why:** The survey showed most "advanced" competitors are cloud-
  first with metered rendering — unmetered, watermark-free, fully
  offline work is itself the sharpest differentiator available.
- **Consequences:** Three competitor staples are NOT adopted because
  they collide with SEAN's recorded decisions: cost estimators/priced
  lists (no-prices rule), cabinet configurators (configurator
  exclusion), photo/LiDAR/AI scan-to-plan (photo/scan exclusion).
  Adopting any of those needs SEAN to explicitly reverse the
  underlying decision.

### Full product & engineering plan adopted

- **Decision:** `docs/PRODUCT_AND_ENGINEERING_PLAN.md` is the project's
  research-backed plan: resource inventory with licenses, product
  scope in three tiers, architecture, offline design, the versioned
  data model (including the designed v2 growth path), risks, and
  milestones M0–M5 with acceptance criteria.
- **Why:** SEAN asked for a senior-architect plan — research first,
  then a practical build plan, then a small first vertical slice.
- **Consequences:** The broader home-design features in that plan
  (multi-room/floors, materials, lighting design, walkthroughs) are
  later milestones only. They do not widen v1, which remains the
  room planner below. Deviating from the plan's stack or format rules
  needs a new entry here.

### Scope: room planner only

- **Decision:** RoomForge is a room planner and nothing more: draw a room
  to scale, place furniture, view it in 2D and 3D, and save/print the plan.
- **Why:** SEAN narrowed the scope explicitly to the room-planner part.
- **Consequences:** Modular product configurators (wardrobe, kitchen and
  shelving builders) are excluded. Photo- and scan-based design of your
  own space is excluded. Neither is planned; adding either would need a
  new decision from SEAN.

### No prices, anywhere

- **Decision:** RoomForge shows no prices — not on catalogue items, not
  as totals, and not in printed or exported lists.
- **Why:** SEAN decided the price part is unnecessary.
- **Consequences:** The catalogue carries names, dimensions and
  descriptions only. No price fields may be added to the data model,
  the user interface, or any export without SEAN reversing this decision.

### Stack: Rust + egui/eframe with the glow renderer

- **Decision:** Rust, using egui via eframe, with the glow renderer
  explicitly selected.
- **Why:** Rust is SEAN's standing default, and egui matches other apps
  in the family. The glow renderer is chosen deliberately: a previous
  family app (a tray downloader built with egui) shipped a wgpu build
  with no backend feature enabled and panicked on real Windows machines.
  The explicit glow feature avoids that failure mode entirely.
- **Consequences:** One language for model, logic and interface. Builds
  must keep the renderer feature explicit in `Cargo.toml`; dropping it
  is a breaking change to be caught in review, not at runtime.

### 3D view: real-time shaded projection in the egui canvas

- **Decision:** The 3D view is a real-time shaded 3D projection rendered
  directly in the egui canvas — orbit, pitch and zoom — not a separate
  game or rendering engine.
- **Why:** The furniture is simple shapes with flat materials; a full
  engine would add heavy dependencies, build time and maintenance for
  no user benefit.
- **Consequences:** Clean shaded 3D with a controllable camera is the
  target. Photorealistic rendering, imported 3D scenes and real-time
  lighting simulation are out of scope.

### Generic, original furniture catalogue

- **Decision:** The catalogue is generic and original — furniture types
  with realistic dimensions, drawn as simple original shapes.
- **Why:** It keeps the project self-contained, freely licensed and
  shippable, with no third-party models, names or data to clear.
- **Consequences:** Items are named by what they are (for example
  "3-seat sofa", "Double bed 160×200"). Users can edit items and add
  their own. No third-party product names or models may be added.

### Project files: versioned JSON, saved atomically

- **Decision:** Plans are `.roomforge` files in versioned JSON, saved
  atomically (write to a temporary file, then replace).
- **Why:** JSON is open, inspectable and easy to share and back up.
  Versioning keeps a migration path when the format grows, and atomic
  writes mean a crash or power loss mid-save cannot leave a torn file.
- **Consequences:** Every format change bumps the version, updates any
  fixtures/golden files in the same change, and keeps a reader or a
  tested migration for the previous version. Files that fail to load
  are reported with a specific error and left untouched on disk, never
  silently discarded.
