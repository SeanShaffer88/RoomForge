# RoomForge — Product & Engineering Plan

Date: 2026-10-22 · Status: v1 plan, research checked 2026-10-22
Owner: Sean Shaffer · License of the product: MIT

## How to read this document

- **Verified** = stated by the cited primary source, checked 2026-10-22.
- **Recommendation** = my engineering judgement, labelled as such.
- **Assumption** = a decision I made because SEAN delegated it
("do whatever you have to do"); each one is listed so it can be reversed.
- Two of SEAN's decisions bind everything below and are **not** open for
redesign: (1) the product is a **room planner** — draw a room to scale
(walls, doors, windows, sockets, pipes), drag-and-drop furniture in 2D,
switch to 3D, save/share/print the plan and item list; (2) **no prices,
anywhere, ever** — no price fields, no totals, in the catalog, the data
model, the UI, or any export. Product configurators (modular wardrobe /
kitchen systems) and photo/scan-based design are out of scope.
- The broader home-design features requested in the planning brief
(multiple rooms and floors, materials, lighting design, walkthroughs)
are planned below as **later milestones**, layered on the room-planner
core — they do not change the v1 scope and are not v1 promises.
- Nothing in this plan claims the program is complete or production
ready. Implementation status is reported separately and honestly,
with Windows-runtime claims labelled Unverified until the app has run
on SEAN's machine.

## Assumptions (stated, reversible)

1. Single-user, single-machine product. No accounts, no cloud, no
telemetry. Sharing means exchanging project files.
2. Small team (effectively one developer + AI assistance): the stack
must minimise specialist surface area — one language, one UI
framework, no server anything.
3. Windows 10 (22H2) and Windows 11, x64 first; ARM64 only if the
toolchain gives it cheaply later.
4. Metric is the internal unit (metres, f64). Imperial is a display and
input concern only.
5. The bundled catalog is original and generic — furniture archetypes
with realistic dimensions, authored for this project. No third-party
product data, names, photos, or models are bundled.
6. A "polished" result means: native-feeling, keyboard-complete,
accessible, forgiving (undo/autosave/recovery), and honest about its
limits — per the Universal Windows Desktop Master Prompt, which
governs this project.

---

# Part A — Research inventory

Checked 2026-10-22 against primary sources where possible. "Maintained"
is judged from release/commit activity visible at check time.

## A.1 Windows desktop frameworks and packaging

| Resource | Official URL | Useful for | Maintained | License | Limitations |
|---|---|---|---|---|---|
| Windows App SDK / WinUI 3 (Microsoft Learn) | https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/migrate-to-windows-app-sdk/migration-decision-guide | Microsoft's recommended native UI for new Windows-only apps; Fluent design, MSIX-first | Yes — Microsoft's active investment | Proprietary platform; SDK terms apply | XAML learning curve; some WPF controls have no equivalent; C++/C# only — no first-party Rust path |
| WPF on modern.NET (Microsoft Learn) | same migration guide, above | Mature XAML UI; Microsoft states you do **not** have to migrate — WPF/WinForms are "fully supported on modern.NET and continue to receive feature updates" | Yes |.NET is MIT-licensed open source | Windows-only; 3D support is dated (needs HelixToolkit, below) |
| MSIX packaging (Microsoft Learn) | https://learn.microsoft.com/en-us/windows/apps/package-and-deploy | Clean install/uninstall, package identity, differential updates; Learn maps distribution goals to packaging/runtime modes and notes any desktop app (WPF/WinForms/Win32) can be packaged in MSIX without code changes | Yes | Platform technology | Signing certificate needed for direct distribution; sideloading friction for unsigned packages; single-project MSIX is WinUI-oriented |
| Inno Setup | https://jrsoftware.org/isinfo.php | Script-driven classic installer; "a free installer for Windows programs" per CMake's official docs | Yes (7.x current in 2026) | Inno Setup License (free incl. commercial use; verify current terms before shipping) | Classic installer model — no package identity, no Store; updates are DIY |
| egui / eframe (Rust) | https://lib.rs/crates/egui | Immediate-mode GUI in pure Rust, runs native and web; AccessKit accessibility integrated (contributor credit, mwcampbell) | Yes — frequent releases (0.33.x in use here; newer lines exist) | "egui is licensed under MIT OR Apache-2.0" | Immediate-mode: you build CAD-style editing yourself; widget set is smaller than XAML's; default fonts carry their own licenses (documented in the crate) |

**Verified platform fact that shapes this project:** Microsoft recommends
WinUI for new Windows-only apps.
**Recommendation (deviation, documented):** RoomForge uses Rust + egui
anyway, because (a) SEAN's standing default is Rust unless a better
option is recommended *for that project*, (b) the core of this product
is custom 2D/3D canvas editing, not standard controls, so the XAML
advantage is small, and (c) one language across model, geometry,
rendering and UI keeps a tiny team fast. The trade-off is real and is
recorded in DECISIONS.md: we give up native XAML controls, the Fluent
control library, and first-party MSIX tooling.

## A.2 3D rendering, scene graphs, model formats

| Resource | Official URL | Useful for | Maintained | License | Limitations |
|---|---|---|---|---|---|
| glTF 2.0 / GLB (Khronos) | https://www.khronos.org/gltf/ | The interchange format for furniture models: "a royalty-free specification for the efficient transmission and loading of 3D scenes and models"; GLB packs scene, meshes, materials, textures into one file; also ISO/IEC 12113:2022 | Yes | Royalty-free specification | A delivery format, not an editor format — parametric data (door swing, drawer rules) does not survive; per-asset licenses still apply to models themselves |
| wgpu (Rust) | https://lib.rs/crates/wgpu (via search: gfx-rs/wgpu) | Safe Rust graphics API over Vulkan/Metal/D3D12/OpenGL — the future high-end renderer path | Yes | MIT OR Apache-2.0 | Low-level: scene graph, materials, shadows are all DIY; **project lesson (verified on SEAN's machine in a sibling app): an eframe/wgpu build with no backend feature compiles, passes CI, and panics at startup on real Windows — the glow/OpenGL path is the safe default here** |
| three-d (Rust) | https://lib.rs/crates/three-d | Higher-level OpenGL renderer: "seeks to make graphics simple but still have the power", targets tools and 2D/3D apps | Yes (0.19, Apr 2026) | MIT | OpenGL ceiling (no modern GPU effects); smaller ecosystem than wgpu |
| Bevy (Rust) | https://bevyengine.org (see licensing summary) | Full ECS game engine: 3D renderer, glTF loading, cross-platform | Yes — very active | "free, open-source, MIT/Apache-licensed… no runtime fees" | No official visual editor; "APIs that break between minor versions"; ECS is the wrong shape for a document editor with undo — heavyweight for this product |
| Godot | https://godotengine.org/license/ | Complete engine + editor, exports Windows builds; "free to use Godot Engine, for any purpose" under MIT | Yes | MIT | An embedded Godot runtime would dwarf the app and fight the document/undo model; better as an asset-authoring tool than as this app's shell |
| HelixToolkit (.NET) | https://github.com/helix-toolkit/helix-toolkit | The standard 3D toolkit for WPF/WinUI (viewports, mesh builders, importers) — the credible.NET route; third-party guidance confirms it "provide[s] higher level abstractions… for 3D graphics in WPF" | Yes | MIT (verify per-repo before use) | Only relevant if the stack flips to.NET; SharpDX lineage adds dependency weight |

## A.3 Geometry, measurements, catalogs, formats

| Resource | Official URL | Useful for | Maintained | License | Limitations |
|---|---|---|---|---|---|
| glam (Rust) | https://lib.rs/crates/glam | SIMD vector/matrix/quaternion math for the 3D layer | Yes | "Apache License, Version 2.0… MIT license… at your option" | f32-first; the document model should stay f64 metres and convert at the render boundary |
| Own SAT geometry (in-repo) | n/a — implemented in `src/geometry.rs` | Oriented-rectangle overlap, point-in-shape, snapping: small, exact, fully tested — for furniture footprints this beats importing a CAD kernel | n/a | Own code, MIT | Rectangles only today; L-shaped rooms and wall-segment graphs need polygon work (ear-clipping / polygon boolean crate) — flagged as a milestone, not assumed |
| IFC / buildingSMART | https://standards.buildingsmart.org/MVD/RELEASE/IFC4/ADD2_TC1/RV1_2/HTML/schema/views/reference-view/index.htm | Pro interoperability: IFC4 is ISO 16739; practical guidance: "for buildings, IFC4 is the safe default" | Yes | Open standard | Enormous schema; a consumer room planner should treat IFC as a late, export-only stretch goal — never the native format |
| Sweet Home 3D (reference study only) | https://www.sweethome3d.com/license/ | Proof of the workflow users expect (2D plan + furniture + 3D visit) and a catalog of UX lessons | Yes | **GPL v2 or later** — the app, its editors and JS version "are distributed under the GNU General Public License"; its bundled models/textures carry their own terms (Free Art License / CC-BY attribution for some) | **Do not bundle or port its code or assets into an MIT product.** Study the UX; write original code. Its per-asset licensing is exactly the trap the brief warns about |
| Poly Haven (assets) | https://polyhaven.com/license | Optional CC0 PBR textures/HDRIs/models for richer materials: "All assets… are licensed as CC0… You can use our assets for any purpose, including commercial work… You can redistribute them" | Yes | CC0 | Download-time only (the app stays offline); each asset still gets a provenance note in the catalog manifest; furniture *models* still need dimension checks before use |
| ambientCG (assets) | https://ambientcg.com | Second CC0 material source (per comparison sources, CC0) | Yes | CC0 — **verify on the official license page before bundling any asset** | Same provenance rule as Poly Haven |

## A.4 Accessibility, usability, testing, distribution

| Resource | Official URL | Useful for | Maintained | License | Limitations |
|---|---|---|---|---|---|
| Microsoft Learn — accessible Windows apps | https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-checklist | The acceptance bar: three pillars — programmatic access (names/roles/values for assistive tech), full keyboard operation, colour/contrast incl. high contrast, with "4.5:1 for normal text, 3:1 for large text" | Yes | Documentation | Written for XAML apps; principles transfer, control-level recipes do not |
| AccessKit | https://accesskit.dev/how-it-works/ | How egui exposes UI Automation on Windows: push-model accessibility tree, adapters at "rough feature parity… enough functionality to make non-trivial applications accessible" | Yes | "licensed under the Apache License, Version 2.0 or the MIT license, at your option"; portions derived from Chromium under BSD-style terms | Canvas-drawn content (the plan itself) is not automatically accessible — the app must expose plan objects as accessible elements and offer full keyboard editing (RoomForge does keyboard nudge/rotate/delete for this reason) |
| cargo test + clippy + CI on windows-latest | https://doc.rust-lang.org/cargo/ | The gate: fmt, clippy `-D warnings` (host + windows-msvc), full test suite, Windows check; CI mirrors it on a real Windows runner | Yes | MIT/Apache (Rust) | CI never *runs* the GUI — startup crashes are only caught by a smoke step and by on-machine runs; never label Windows behaviour Verified from CI alone |
| Fluent 2 Design System | https://fluent2.microsoft.design | Visual/interaction reference for a native-feeling Windows app | Yes | Microsoft design guidance | Reference only for a non-XAML app; apply principles (hierarchy, spacing, states), not assets |

**Licensing red flags (explicit, per the brief):** IKEA's catalog data,
product names, models and prices are proprietary — none may be used;
that is also why the catalog is original. Sweet Home 3D code/assets are
GPL/CC-BY — study-only. Any downloaded model or texture, however
"free" the site looks, needs a per-asset license check recorded in the
catalog manifest before it ships. Engine/runtime licenses above are
permissive (MIT/Apache/CC0) as cited.

---

# Part B — The plan

## 1. Product scope

**Target users.** (a) Homeowners/renters planning a room before moving
furniture or buying anything; (b) landlords and stagers sketching
layouts; (c) DIY renovators checking fit (door swings, radiator
clearances, socket positions). Not architects: no construction
documentation, no structural or code-compliance claims.

**Core use cases.** Measure a real room → reproduce it to scale →
place the fixed services (sockets, pipes, radiators) → try furniture
layouts safely → see it in 3D → print/share the plan and the item list.

**Priorities.**

- **MVP (v1, this build):** one rectangular room to scale; doors and
windows on any wall with position/width/height/sill; sockets, pipes,
radiators; ~49-piece generic catalog; 2D drag-and-drop with snap,
rotate, resize, duplicate; collision/fit warnings; synchronised 3D
orbit view; undo/redo; autosave + crash recovery; save/open
`.roomforge` files; export SVG plan, printable HTML sheet, CSV item
list; fully offline; keyboard-complete editing.
- **Next release (v1.x):** wall-segment editor (L-shaped and arbitrary
rooms), polygon footprints, imperial display/input, materials &
colours per surface with CC0 texture pack, PNG render export,
dimension lines on the plan, multi-room projects (rooms side by
side, shared project file v2), catalog search/favourites and custom
item dimensions, still-image "photo" render with simple lighting.
- **Advanced (v2+):** multiple floors with stair openings, ceiling
elements and lighting design (fixtures, switches, day/night
preview), first-person walkthrough, annotations and measurement
overlays, GLB import of user furniture models (with per-file license
prompts), IFC4 export (reference view subset), template rooms.

## 2. User workflows

1. **Create a room:** New → starter room (5 × 4 m) or blank → set
width/depth/ceiling height numerically or by dragging in the Room
panel → pick floor/wall colours.
2. **Draw/import a floor plan:** v1: type measured dimensions (tape
measure workflow, one room). Later: trace over a scanned plan image
with a two-point scale calibration; wall-segment drawing with
length/angle readouts as you draw.
3. **Doors/windows:** pick the wall in the Fittings panel → Add door /
window → drag along the wall numerically (offset) → set width,
height, sill, swing side; the plan shows the swing arc and the 3D
view cuts/glosses the opening.
4. **Furnish:** browse/search the catalog (left panel) → click an item,
click in the room (ghost preview first) → drag to move, R to
rotate, arrow keys to nudge, properties panel for exact numbers →
warnings flag overlaps and covered radiators/pipes immediately.
5. **Materials & lighting (v1.x):** select a surface or item → assign
material/colour → 3D view updates; lighting presets (day/evening)
change the preview shading.
6. **View in 3D:** press 2 or the 3D toggle → orbit by dragging,
sliders for yaw/pitch/zoom → same document, no export step; edits
in either view are the same edit (single source of truth).
7. **Save/export:** Ctrl+S saves `.roomforge` (atomic write); autosave
protects against crashes and is offered for recovery on next
launch; share by sending the file; print via the self-contained
HTML sheet (plan + item list + fittings schedule); export SVG for
vector use and CSV for the item list. **No export contains prices.**

## 3. Architecture

**Chosen stack (Recommendation, implemented for v1):**

- **Language/UI:** Rust + egui/eframe, **glow (OpenGL) renderer pinned**
(see A.1/A.2 for why, including the verified wgpu startup-panic
lesson). Single binary, no runtime to install.
- **2D editor:** egui `Painter` canvas over the library's geometry:
screen↔world transform in one place, 0.5 m grid, 5 cm snap toggle,
SAT overlap tests on rotated footprints.
- **3D engine:** a small in-repo perspective renderer (projection maths
in the library, painter's-algorithm faces, per-face shading) for v1 —
deliberately modest; the upgrade path is three-d (MIT) or wgpu when
real materials/lighting land in v1.x, behind the same `view3d`
interface so the document model never changes.
- **Geometry layer:** pure Rust library functions — OBB overlap,
point-in-OBB, snapping, clamping; all unit-tested off-Windows.
- **Catalog system:** compile-time catalog of original items
(id, name, category, W×D×H, colour) in v1; v1.x moves to a data file
+ manifest with per-asset license/provenance fields and optional
local asset packs (folders of GLB + thumbnails) that the app scans
at startup — still fully offline.
- **Project storage:** versioned JSON (see §5), atomic writes,
autosave beside the app data folder, recovery prompt.
- **Rendering/exports:** SVG plan (to scale), print HTML, CSV — all
generated by the library, so they are testable headlessly.
- **Installer:** v1 ships a zip of the single exe (honest, simple);
the release path is Inno Setup (free, script-driven) or MSIX once a
signing certificate decision is made — MSIX gives cleaner updates
and identity but needs signing;
Inno works unsigned with the usual SmartScreen caveat.

**Alternatives compared.**

| Option | Strengths | Why not (for this team/product) |
|---|---|---|
| C# + WinUI 3 + HelixToolkit | Microsoft-recommended, best native controls, MSIX-first | Second stack for SEAN's portfolio; 3D via ageing SharpDX lineage; slower iteration for a one-dev team already standardised on Rust |
| C# + WPF + HelixToolkit | Most mature XAML, fully supported | Same trade as above; WPF 3D is legacy without Helix |
| Rust + Bevy | Powerful renderer, glTF built in | ECS fights document/undo architecture; breaking APIs each release |
| Rust + wgpu direct | Modern GPU ceiling | Weeks of renderer plumbing before any product; plus the verified startup-panic class of failure on real machines |
| Tauri/Electron + three.js | Huge web ecosystem, fast UI | WebView/runtime weight, weaker offline file semantics, a second language/runtime to secure and ship; three.js is excellent but the shell is the cost |
| Godot shell | MIT, editor included | App becomes a game-engine project; document model, printing, accessibility all uphill |

## 4. Offline design

- **Core rule:** every design, save, render, reopen, print and export
path works with the network cable out. There is no account system,
no licence server, no cloud sync, no telemetry — nothing to fail
when offline because nothing phones home at all.
- **Catalog:** ships inside the binary (v1) or the install folder
(v1.x asset packs); updates arrive only with app updates or by the
user dropping a pack folder in — the app never downloads assets.
- **Projects:** single-file JSON the user owns; copy it, email it,
put it on a USB stick — it opens anywhere RoomForge runs. Embedded
assets (v1.x textures) travel as an optional project bundle (zip).
- **Updates:** manual and user-initiated (download a new installer).
An *optional* update check, if ever added, must be off-switchable,
must never block startup, and is listed here in advance as the only
planned network feature.
- **Installation:** the installer itself must work offline (Inno/MSIX
sideload both do); the one caveat to document honestly is Windows
SmartScreen behaviour for unsigned builds.

## 5. Data model

**Format:** `.roomforge`, UTF-8 JSON, `format_version` integer.
Canonical internal units: metres (f64). All structs tolerate missing
optional fields (defaults) and reject newer versions with a plain
error ("saved by a newer version of RoomForge").

**v1 schema (implemented):**

- `Project { format_version, name, room, openings[], services[], items[], next_id}`
- `Room { name, width_m, depth_m, height_m, wall_rgb, floor_rgb}`
(clamped 1–30 m across, 1.8–6 m ceiling on load)
- `Opening { id, kind: door|window, wall: north|south|east|west,
offset_m (along wall from its start), width_m, height_m, sill_m,
swings_in}` — clamped to the wall on load.
- `ServicePoint { id, kind: socket|pipe|radiator, wall, offset_m}` —
sockets/pipes/radiators live *on* walls; footprint sizes are
constants per kind.
- `PlacedItem { id, catalog_id, name, x_m, y_m (centre, room coords,
origin SW), width_m, depth_m, height_m, rotation_deg, rgb}` —
items are self-describing: a project stays meaningful even if the
catalog changes.

**v2 schema (planned, designed now so v1 doesn't paint us in):**
`floors[]` each with `rooms[]`; rooms gain `origin` offsets; walls
become segment lists with per-segment openings; `materials{}` table
referenced by id from surfaces/items; `lights[]`; `cameras{}` (named
saved views); `annotations[]`. v1 files read as a one-floor,
one-room v2 document.

**Migration rules:** (1) migrations are pure functions
`vN → vN+1` in the library, chained on load; (2) every migration ships
with golden fixtures (a real vN file + expected vN+1 output) in the
test suite — the full suite, never a subset, guards them; (3) loading
never destroys: out-of-range values are repaired by clamping (and the
repair is total — re-saving a repaired file is stable); (4) the app
refuses files from the *future* rather than mangling them.

## 6. Feature details

- **Dimensions/units:** everything stored in metres; UI shows metres
with cm precision in v1; imperial (ft/in, fractional) is a display
layer in v1.x — storage never changes. Plans print to scale with a
scale bar and room area in m² (ft² later).
- **Walls/floors/ceilings:** v1 room is a rectangle with per-project
wall/floor colours and ceiling height; openings are cut visually in
3D (inset glass for windows, gap + swing leaf for doors). v1.x:
wall segments (draw, drag endpoints, type exact lengths), floor
shapes from the wall polygon, ceiling on/off per room.
- **Snapping/alignment:** 5 cm grid snap (toggle), snap to room edges
and to other items' edges (v1.x guides), arrow-key nudge 1 cm
(Shift = 25 cm), numeric fields everywhere a mouse action exists.
- **Furniture:** place/move/rotate (15° steps, Shift = 90°)/resize via
numeric W/D/H (footprint changes, height is presentational in 2D,
real in 3D), duplicate, delete, multi-place with Shift.
- **Collision/fit:** rotated-rectangle SAT overlap → live warnings
list + red outline; furniture over radiators/pipes warns (heat/
access), over sockets does not (normal in real rooms — a deliberate
product decision); door-swing clearance check lands with the
wall-segment milestone.
- **Multiple rooms/floors:** v2 per §5; until then, one project file
per room is the honest workflow and the UI says so.
- **Materials:** v1 per-item/per-surface colours; v1.x material
library (paint, wood, tile, fabric) with CC0 texture packs and
per-surface assignment.
- **Lighting:** v1 fixed pleasant shading; v1.x day/evening presets,
ceiling/wall fixtures as catalog items with light cones in 3D,
still-image render at print quality.
- **Camera:** orbit (yaw/pitch/zoom) in v1; saved named views and a
first-person walk mode are v2.
- **Annotations:** v2 — text notes pinned to positions, measurement
overlays; both stored in the project file.
- **Exports:** SVG (vector plan), print HTML (plan + item list +
fittings schedule), CSV item list, project file itself. PNG render
in v1.x, GLB scene export and IFC4 as advanced stretch goals.
**No export contains prices or totals of any kind.**

## 7. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Performance (large rooms, many items) | The scene is tiny by GPU standards; the real cost is immediate-mode redraw — cap repaint work, keep geometry tests O(n²) only over placed items (n is small), profile before optimising |
| Large catalogs later | Data-driven catalog + manifest with lazy thumbnails; search-first UI from the start; asset packs on disk, not in the binary |
| Geometric edge cases | One geometry module, property-style tests (round-trips, clamp stability, touching ≠ overlapping), golden project fixtures; rotations stored in degrees and normalised |
| Usability (CAD tools scare people) | Starter furnished room on first run, ghost previews, forgiving undo (100 steps), plain-language labels, tooltips on every control, no essential info only in tooltips |
| Licensing | Original catalog only; per-asset provenance manifest; GPL/CC-BY sources are study-only; engine deps all MIT/Apache/CC0 as cited in Part A |
| Offline installation | Single-exe zip for v1; Inno/MSIX later; document the unsigned-build SmartScreen caveat instead of hiding it |
| Scope creep (configurators, scanning, cloud) | Out of scope by SEAN's recorded decision; new ideas go to the v2 list in §1, not into the current milestone |
| "CI green ≠ works" | A CI smoke step + SEAN's on-machine run is the only Windows verification; all status reports label runtime claims Unverified until then |
| Renderer ceiling | The in-repo 3D renderer is behind a library interface; three-d/wgpu can replace it without touching the document model |

## 8. Milestones (testable acceptance criteria)

- **M0 — Vertical slice (v0.1):** resizable window; 2D editor with
editable dimensions; synchronised 3D preview; ≥1 placeable item
(in fact 49); undo/redo; offline save/load. *Accept:* the full gate
(fmt, clippy host + windows-msvc `-D warnings`, full tests, Windows
check) is green; a plan saved and reopened is byte-stable in
content; **on-machine run by SEAN still required before any
"works on Windows" claim.**
- **M1 — Planner completeness:** fittings (doors/windows/sockets/
pipes/radiators) fully editable; warnings; exports (SVG/HTML/CSV).
*Accept:* export tests assert the schedule contents and the absence
of prices; print sheet renders standalone in a browser.
- **M2 — Rooms beyond rectangles:** wall-segment editor, L-shaped
rooms, door-swing clearance, format v2 + migration goldens.
*Accept:* every v1 fixture migrates and re-saves losslessly.
- **M3 — Materials & light:** material library, CC0 texture pack with
provenance manifest, day/evening lighting, PNG render export.
*Accept:* a fresh install offline renders the demo room with
textures; manifest lists a license for every bundled asset.
- **M4 — Whole-home:** multiple rooms, then floors; annotations;
saved camera views. *Accept:* a 3-room project round-trips; area
totals per floor shown (areas, never prices).
- **M5 — Interchange (stretch):** GLB import/export, IFC4 export
subset. *Accept:* exported GLB opens in a third-party viewer;
IFC validates against the buildingSMART reference-view schema.

## 9. Starter implementation (the first vertical slice)

**Status note:** this slice is exactly M0+M1 above; it has been built
as RoomForge v0.1 under SEAN's "do whatever you have to do" instruction
and is being gated and published; Windows runtime remains Unverified
until SEAN runs it.

**Folder structure (as built):**

```
roomforge/
├─ Cargo.toml # egui/eframe 0.33 (glow pinned), serde, rfd (Windows)
├─ src/
│ ├─ lib.rs # library root — all decision logic lives here
│ ├─ model.rs # versioned project schema + migrations/repairs
│ ├─ geometry.rs # SAT overlap, point-in-OBB, snapping
│ ├─ catalog.rs # original generic catalog (no prices, ever)
│ ├─ view3d.rs # projection maths (pure, testable)
│ ├─ export.rs # SVG plan, print HTML, CSV list
│ ├─ io.rs # atomic save/load, autosave path
│ ├─ app.rs # egui GUI — renders state, collects intent only
│ └─ main.rs # glow renderer, panic-to-log hook, CLI file arg
├─ docs/
│ ├─ PRODUCT_AND_ENGINEERING_PLAN.md # this file
│ └─ screenshots/CAPTURE.md # real-machine capture list
├─.github/workflows/ci.yml # fmt, clippy -D warnings, tests, release build
├─ DECISIONS.md AGENTS.md CHANGELOG.md README.md LICENSE (MIT)
```

**First implementation tasks (done for v0.1, in order):** (1) schema +
round-trip tests before any UI; (2) geometry with SAT tests;
(3) catalog with dimension sanity tests; (4) 2D canvas + placement;
(5) 3D projection as pure functions, then the painter; (6) undo as
document snapshots; (7) atomic IO + autosave recovery; (8) exports as
library functions with content tests; (9) the gate, then CI, then —
and only then — an on-machine verification pass by SEAN.
