# RoomForge — Architecture & UX Plan (SUPERSEDED 2026-10-22)

> **Superseded.** SEAN decided the scope on 2026-10-22: room planner only,
> **no prices anywhere**, no product configurators, no photo/scan design.
> The current plan is `docs/PRODUCT_AND_ENGINEERING_PLAN.md`. This early
> draft is kept for history only — its priced-list and configurator
> sections must not be implemented.

# RoomForge — Architecture & UX Plan (DRAFT, awaiting SEAN decisions)
Date: 2026-10-22 · Status: Pre-build, ~70% certainty — decisions in §7 needed to reach 95%
Working name: **RoomForge** (fits FileWeaver / ReelDeck family; rename freely)

## TL;DR
An offline Windows desktop room designer that does what IKEA's online planners do —
draw a room to scale, place furniture in 2D and 3D, configure modular storage/kitchen
systems by rules, and get a priced parts/shopping list — without needing a browser,
an IKEA account, or an internet connection. It cannot ship IKEA's own 3D models,
photos, or live prices (those are IKEA's), so it ships an original generic catalog
with the same *systems* (wardrobe, kitchen, shelving) and lets users add/edit items.

## 1. What "everything IKEA's tool does" actually is — Verified from ikea.com

IKEA does not have one tool. It has three families:

### A. Room planners (general)
- Draw a room: custom wall lengths, ceiling height, room shapes (rect / L-shape)
- Place doors, windows, radiators/obstacles, sockets, pipes/water, electrical
- Drag-and-drop furniture from a catalog into a scaled 2D floor plan
- Switch 2D ⇄ 3D, walk/orbit the 3D view, check sight lines
- Collision / clearance rules (doors & drawers can't be blocked, standard gaps)
- Wall/floor colours and materials per surface
- Save design + get a design code/link, share/remix, print plan + product list
- Live budget: total price updates as you place items

### B. Product planners / configurators (modular systems)
Separate planners for each system — kitchen (METOD, ENHET, KNOXHULT),
PAX, PLATSA, BESTÅ, KALLAX, BOAXEL, IVAR, EKET, BILLY, SMÅSTAD, BROR, JONAXEL, SKYTTA, sofas, etc.
- Pick frames by width/height/depth, then doors, drawers, shelves, rails, organisers
- Rules engine: only compatible parts fit; required hardware auto-added
- Exploded parts list: every frame, hinge, slide, handle accounted for, with prices
- Add-all-to-cart / shopping list, stock-style checklist

### C. IKEA Kreativ (visual / AR-style)
- Ready-made 3D showrooms you can personalise (swap/remove/move products)
- Design your own space from a photo/scan: erase existing furniture, place
  true-to-scale 3D products with matched lighting
- Room Builder on web (custom dimensions), save/share designs

Sources: ikea.com planners index and support articles (checked 2026-10-22);
IKEA Kreativ mobile-vs-web differences article; kitchen/PAX planner guides.

### Honest exclusions for a desktop clone
- Live IKEA stock, prices, cart/checkout, and IKEA account cloud save — no public
  IKEA API for this; we do local save + shareable design files/codes instead.
- Phone LiDAR room scanning / AR — that's the mobile app, not the web tool,
  and not available to a Windows desktop app. Photo-based design is deferred (§6).
- IKEA's proprietary 3D models/photos/names — we build original low-poly models
  and a generic catalog. Users can rename items and enter real dimensions/prices.

## 2. Proposed phasing (one product, three milestones)

**v0.1 — Room Planner core (the 80% tool)**
Room editor (walls, doors, windows, obstacles, sockets), measurements in
mm/cm + ft/in, 2D plan with snap/grid/dimension lines, catalog browser
(~120 original generic items: beds, sofas, tables, chairs, storage, kitchen,
bathroom, lighting, decor), drag/move/rotate/resize, collision warnings,
2D ⇄ 3D (orbit + first-person walk, wall/floor materials, day lighting slider),
undo/redo, autosave, project files (.roomforge JSON, versioned), shopping/parts
list with editable prices + total, print/export plan (PDF/PNG) and list (CSV/PDF),
room templates + 6 furnished showroom rooms. Black/red family theme.

**v0.2 — Modular configurators**
Wardrobe configurator (PAX-style), kitchen configurator (METOD-style),
shelving configurator (KALLAX/BESTÅ-style): frame picker, interior fittings,
compatibility rules, auto hardware, exploded parts list feeding the same
shopping list. Configured units drop into the room planner as one object.

**v0.3 — Kreativ-style**
Start from a room photo (perspective backdrop + scaled items), erase/replace
workflow, share/remix via exported design files, more materials/lighting
(time-of-day, lamp lights), screenshot renders for the README/showrooms.

## 3. Stack — checked options

Standing default is Rust. For 2D CAD + real-time 3D, the realistic Rust routes:

| Option | Verdict |
|---|---|
| **Rust + egui/eframe + wgpu (custom 3D viewport)** | **Recommended.** Same family as ytTrayDL (egui). One language, native exe, offline, Win10+ feasible. 3D is simple shaded low-poly — well within wgpu. Risk: egui accessibility is weaker than native controls; mitigate with keyboard-first design + accessible names where egui supports them, and honest limits. |
| Rust + Bevy | Real game engine — powerful, but heavy, slower builds, UI/accessibility weaker, overkill for furniture blocks. Reject for v1. |
| C# WPF + HelixToolkit3D | Best native accessibility + fastest 3D toolkit on Windows (MuseDesktop precedent for C#). Cost: breaks Rust default + family tooling (WinDevKit). Viable fallback if accessibility is the top priority. |
| Tauri + Three.js (web UI) | Fastest 3D ecosystem (Three.js), easiest pretty renders. Cost: WebView app, not native-feel per charter, Rust default effectively bypassed. Reject unless SEAN wants web-tech speed over native. |

Data: serde JSON project files (versioned, atomic write, migration-tested per
Master Prompt §11), catalog as editable JSON so users/community can extend it.
Packaging: portable zip + Inno Setup installer (ytTrayDL precedent), MIT, unsigned v1
with SmartScreen note, CI on windows runner: fmt + clippy + full tests.

Targets: Windows 10 22H2 + Windows 11, x64. No admin, no telemetry, fully offline.

## 4. UX structure
- Left: catalog (search, categories, favourites) · Centre: 2D plan / 3D view toggle
- Right: properties for the selected item/wall (dimensions, material, price)
- Top: room setup, undo/redo, save, shopping list (live total), export/print
- Bottom status strip: measurements, warnings (collision, door swing, clearance)
- Every control tooltip'd; essentials never tooltip-only; full keyboard flow:
  arrows nudge, R rotate, Del delete, Ctrl+Z/Y, 1/2/3 switch 2D/3D/List views.

## 5. Quality / safety notes (Master Prompt)
No destructive system operations — high-impact protocol is just file overwrite:
autosave + backup copy before save, undo everywhere, corrupt-file quarantine.
Performance budget: cold start < 2s, 60fps orbit with 200 items on integrated GPU —
measured on SEAN's machine before any claim (Unverified until then).

## 6. Deferred / risks
- Photo-based Kreativ design (v0.3) is the weakest-fit feature on desktop; if it
  matters most to SEAN, re-order phases.
- Catalog depth is content work, not code: v0.1 ships ~120 items; configurator
  systems add hundreds of parts in v0.2.
- Photoreal rendering is a non-goal; clean shaded 3D with shadows is the target.

## 7. Decisions needed from SEAN (to reach 95%)
1. **Catalog:** generic/original (recommended, shippable, MIT) — or IKEA-named items
   with hand-entered dims/prices and links out (personal-use feel, trademark risk
   if published)?
2. **First milestone:** Room Planner core first (recommended) — or kitchen/wardrobe
   configurator first — or Kreativ photo/showroom first?
3. **Stack:** Rust + egui/wgpu (recommended) — or C# WPF for best accessibility?
4. **Name/repo:** RoomForge, public GitHub repo SeanShaffer88/RoomForge, MIT —
   or a different name / private for now?
