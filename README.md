# RoomForge

## TL;DR

RoomForge helps you plan a room on your Windows computer before you move
any real furniture. Draw your room to its real size, put in the doors,
windows, sockets and pipes, then drag furniture around on the plan until
it fits the way you like. Flip to a 3D view to see how it will actually
look, and print out the plan and a list of everything in it. It works
completely offline, and there are no prices anywhere in it — by design.

## What it does

- **Draw a room to scale** — set the real width, depth and ceiling height
  of a rectangular room. (L-shaped and free-form walls are planned.)
- **Add the fixed details** — doors, windows, electrical sockets,
  pipes and water points, and radiators, all placed where they really are.
- **Drag-and-drop furniture in a 2D plan** — pick furniture from the built-in
  catalog, drop it in, move it, rotate it, and nudge it into place.
  Dimension labels are shown as you work, and you are warned when items
  overlap or when furniture covers a radiator or a pipe.
- **Switch to a 3D view** — see the same room in 3D, orbit around it,
  tilt it and zoom in, to check how the layout feels before you commit to it.
- **Save, open and share plans** — plans are saved as `.roomforge` files
  you can keep, back up, copy to another computer, or send to someone else.
- **Print and export** — export the plan as a to-scale SVG, print it from
  the generated print sheet, and export the full list of items as CSV.

There are **no prices** in RoomForge: no prices on items, no totals, and
no prices in the printed or exported lists. That is a deliberate design
decision — RoomForge is for planning a layout, not for shopping.

## Features

- Room editor with walls, doors, windows, sockets, pipes/water points
  and radiators
- Measurements in metres (imperial display is planned), with snap-to-grid
  and dimension labels
- Furniture catalog covering beds, seating, tables, storage, kitchen,
  bathroom, lighting and decor — every item with real-world dimensions
- Move, rotate, duplicate and delete items, with full undo and redo
- Warnings for overlapping items and for furniture covering radiators
  or pipes
- 2D plan view and real-time 3D view of the same room
- Item list panel listing everything placed in the room
- Autosave, so work is not lost if the program closes unexpectedly
- Export the plan as SVG, a printable plan + list sheet as HTML, and the
  item list as CSV
- Works fully offline — no account, no sign-in, no data collection
- Black and red family theme, designed to be usable with keyboard alone

## How to build and run

You will need a current stable Rust toolchain.

```sh
cargo run --release
```

To build without running:

```sh
cargo build --release
```

The program targets Windows 10 and Windows 11 (x64). No administrator
rights are needed.

## Project files

Plans are saved as `.roomforge` files. The format is versioned JSON,
written safely (a new file is written first, then swapped in), so an
interrupted save should not damage an existing plan. Because the files
are plain JSON, they are easy to back up, share and inspect.

## Keyboard shortcuts

| Keys | Action |
| --- | --- |
| Arrow keys | Nudge the selected item |
| `R` | Rotate the selected item |
| `Delete` | Delete the selected item |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+S` | Save the plan |
| `1` | 2D plan view |
| `2` | 3D view |
| `3` | Item list view |

## Screenshots

Screenshots will show the main window in the 2D plan view, the 3D view,
the item list panel, and the room setup panel. They are taken from real
runs on a Windows machine — the exact list of captures needed is in
[docs/screenshots/CAPTURE.md](docs/screenshots/CAPTURE.md).

## License

MIT — see [LICENSE](LICENSE).
