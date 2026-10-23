# Decisions

A running log of significant decisions for RoomForge: what was decided,
why, and what it means. Newest entries first within each date.

## 2026-10-23

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
