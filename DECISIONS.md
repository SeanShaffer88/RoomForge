# Decisions

A running log of significant decisions for RoomForge: what was decided,
why, and what it means. Newest entries first within each date.

## 2026-10-22

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
