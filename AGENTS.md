# AGENTS.md — RoomForge

Notes for agents working on this project. The Universal Windows Desktop
Master Prompt applies to this project in full:
`~/workspace/your_files/windows-desktop-master-prompt.md`

## Project in one line

A Windows desktop room planner in Rust: draw a room to scale (walls,
doors, windows, sockets, pipes/water points, radiators), drag-and-drop
furniture in 2D, switch to a real-time 3D view, save/share/print the
plan and the item list. MIT. **No prices, anywhere — ever** (see below).

## Hard rules

- **No prices.** No price may ever be added to the catalogue, the data
  model, the UI, or any print/export, without SEAN explicitly reversing
  his 2026-10-22 decision (recorded in `DECISIONS.md`). If a change
  introduces a price field or a total, stop and flag it.
- **Generic catalogue only.** Furniture is original and generic — no
  third-party product names, models, photos or data.
- **Scope.** A room planner plus (since SEAN's explicit reversal
  on 2026-10-25) a furniture configurator: parametric pieces built
  from panels, with a cut list of dimensions and quantities.
  Photo/scan-based design remains excluded (SEAN, 2026-10-22),
  and the no-prices rule above is untouched by the reversal.
- **Never claim Windows runtime is verified** until the build has
  actually been run on SEAN's Windows machine and he (or his log)
  confirms it. Passing local tests, CI, or a Wine run is *not* Windows
  verification. Label claims Verified / Partially Verified / Unverified.
- Public documentation (README, CHANGELOG, guides) must not mention or
  compare competitor software.

## Architecture

- **All decision logic lives in the library** and is testable off-Windows:
  the data model, the catalogue, geometry (placement, rotation, overlap
  and clearance checks, measurements, the 3D projection maths), and
  export/print rendering.
- **The GUI renders state and collects intent only.** It draws what the
  library describes and forwards user actions back to the library. No
  business rules, geometry or validation in GUI code.
- Project files are versioned JSON (`.roomforge`), written atomically
  (write temporary file, then replace). Format changes bump the version
  and update fixtures/goldens in the same change.
- Renderer: egui via eframe with the **glow** renderer feature explicitly
  selected. Do not drop or change that feature casually — a wgpu build
  with no backend feature panicked on real Windows machines in a sibling
  project.

## Design tokens (black/red family look)

- Background: `#000000` / `#101010`
- Hairlines / borders: `#242424`
- Primary red: `#6B0700`
- Red text / accent text: `#E5484D`
- Text: `#F5F5F5` (main), `#A8A8A8` (muted), `#6E6E6E` (faint)

Semantic status colours must always be paired with a text, icon or
position cue — never colour alone. Every interactive control gets a
concise tooltip, and no essential information may live only in a tooltip.

## The gate (run in full before publishing anything)

```sh
cargo fmt --all
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo test --locked
cargo check --target x86_64-pc-windows-msvc --all-targets
```

- Run the **full** test suite, never a filtered subset — goldens and
  defaults tests elsewhere in the suite catch format/default changes.
- If any gate step fails, fix it on the branch; do not publish a red gate.
- CI (`.github/workflows/ci.yml`) mirrors the essentials on
  `windows-latest`: format check, clippy with `-D warnings`, full tests,
  and a release build.

## Documentation

- `README.md` always opens with a `## TL;DR` in plain, jargon-free
  language, and includes a screenshots section. Screenshots come only
  from real runs on SEAN's machine — never placeholders or stock images;
  the capture list is `docs/screenshots/CAPTURE.md`.
- Record significant decisions in `DECISIONS.md` (date, decision, why,
  consequences) and user-facing changes in `CHANGELOG.md`.

## License

MIT — see `LICENSE`. Copyright 2026 Sean Shaffer.
