//! RoomForge GUI. Renders state and collects intent only — every rule
//! (geometry, grouping, warnings, projection, export) lives in the
//! library. There are no prices anywhere in this UI, by design.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use roomforge::catalog::{catalog, categories, CatalogItem};
use roomforge::geometry::{point_in_obb, snap};
use roomforge::model::{
    Opening, OpeningKind, PlacedItem, Project, ServiceKind, ServicePoint, Wall,
};
use roomforge::view3d::{box_corners, box_faces, project as project3d, shade, Camera};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Plan,
    ThreeD,
    List,
}

pub struct App {
    project: Project,
    view: View,
    camera: Camera,
    selected_item: Option<u64>,
    selected_opening: Option<u64>,
    selected_service: Option<u64>,
    placing: Option<CatalogItem>,
    wall_choice: Wall,
    catalog_search: String,
    snap_enabled: bool,
    undo: Vec<Project>,
    redo: Vec<Project>,
    current_path: Option<PathBuf>,
    status: String,
    dirty: bool,
    last_autosave: Instant,
    drag_item: Option<u64>,
    drag_offset: (f64, f64),
    drag_snapshot_done: bool,
}

const RED: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
const PANEL_BG: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A starter room so the first launch shows what the program does.
fn starter_project() -> Project {
    let mut p = Project::default();
    let add = |p: &mut Project, cat_id: &str, x: f64, y: f64, rot: f64| {
        let c = catalog().into_iter().find(|c| c.id == cat_id).unwrap();
        let id = p.alloc_id();
        p.items.push(PlacedItem {
            id,
            catalog_id: c.id.to_string(),
            name: c.name.to_string(),
            category: c.category.to_string(),
            x_m: x,
            y_m: y,
            width_m: c.width_m,
            depth_m: c.depth_m,
            height_m: c.height_m,
            rotation_deg: rot,
            rgb: c.rgb,
        });
    };
    add(&mut p, "rug-large", 1.6, 1.4, 0.0);
    add(&mut p, "sofa-3", 1.6, 2.2, 180.0);
    add(&mut p, "coffee-table", 1.6, 1.35, 0.0);
    add(&mut p, "tv-bench", 1.6, 0.35, 0.0);
    add(&mut p, "dining-table-4", 4.0, 2.6, 0.0);
    add(&mut p, "dining-chair", 3.6, 1.95, 0.0);
    add(&mut p, "dining-chair", 4.4, 1.95, 0.0);
    add(&mut p, "floor-lamp", 0.35, 2.6, 0.0);
    add(&mut p, "bookcase", 4.75, 1.2, 270.0);
    let oid = p.alloc_id();
    p.openings.push(Opening {
        id: oid,
        kind: OpeningKind::Door,
        wall: Wall::South,
        offset_m: 4.1,
        width_m: 0.9,
        sill_m: 0.0,
        height_m: 2.1,
        swings_in: true,
    });
    let wid = p.alloc_id();
    p.openings.push(Opening {
        id: wid,
        kind: OpeningKind::Window,
        wall: Wall::North,
        offset_m: 1.2,
        width_m: 1.8,
        sill_m: 0.9,
        height_m: 1.2,
        swings_in: false,
    });
    let sid = p.alloc_id();
    p.services.push(ServicePoint {
        id: sid,
        kind: ServiceKind::Socket,
        wall: Wall::West,
        offset_m: 1.0,
    });
    let rid = p.alloc_id();
    p.services.push(ServicePoint {
        id: rid,
        kind: ServiceKind::Radiator,
        wall: Wall::North,
        offset_m: 3.6,
    });
    p
}

impl App {
    pub fn new() -> Self {
        // Recover an autosave if one exists (crash / closed without saving).
        let (project, status) = match roomforge::io::load(&roomforge::io::autosave_path()) {
            Ok(p) if !p.items.is_empty() => (
                p,
                "Recovered your last session from the autosave.".to_string(),
            ),
            _ => (
                starter_project(),
                "Welcome to RoomForge. Pick furniture on the left, click in the room to place it."
                    .to_string(),
            ),
        };
        App {
            project,
            view: View::Plan,
            camera: Camera::default(),
            selected_item: None,
            selected_opening: None,
            selected_service: None,
            placing: None,
            wall_choice: Wall::South,
            catalog_search: String::new(),
            snap_enabled: true,
            undo: Vec::new(),
            redo: Vec::new(),
            current_path: None,
            status,
            dirty: false,
            last_autosave: Instant::now(),
            drag_item: None,
            drag_offset: (0.0, 0.0),
            drag_snapshot_done: false,
        }
    }

    /// Open a plan file passed on the command line.
    pub fn open_path(&mut self, path: PathBuf) {
        self.open_from(path);
    }

    fn snapshot(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            let cur = std::mem::replace(&mut self.project, prev);
            self.redo.push(cur);
            self.selected_item = None;
            self.mark_dirty();
            self.status = "Undone.".into();
        }
    }

    fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let cur = std::mem::replace(&mut self.project, next);
            self.undo.push(cur);
            self.selected_item = None;
            self.mark_dirty();
            self.status = "Redone.".into();
        }
    }

    // ---------- file operations ----------

    fn save_to(&mut self, path: PathBuf) {
        match roomforge::io::save_atomic(&path, &self.project) {
            Ok(()) => {
                self.status = format!("Saved to {}", path.display());
                self.current_path = Some(path);
                self.dirty = false;
            }
            Err(e) => self.status = e,
        }
    }

    fn open_from(&mut self, path: PathBuf) {
        match roomforge::io::load(&path) {
            Ok(p) => {
                self.project = p;
                self.undo.clear();
                self.redo.clear();
                self.selected_item = None;
                self.status = format!("Opened {}", path.display());
                self.current_path = Some(path);
                self.dirty = false;
            }
            Err(e) => self.status = e,
        }
    }

    fn export_to(&mut self, path: PathBuf, contents: &str, what: &str) {
        match std::fs::write(&path, contents) {
            Ok(()) => self.status = format!("{what} written to {}", path.display()),
            Err(e) => self.status = format!("Could not write {}: {e}", path.display()),
        }
    }
}

/// File-dialog helpers. Windows uses the native dialog; elsewhere we
/// fall back to a file in the home directory and say where it went.
#[cfg(windows)]
fn save_dialog(filter_name: &str, ext: &str, file_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(filter_name, &[ext])
        .set_file_name(file_name)
        .save_file()
}

#[cfg(not(windows))]
fn save_dialog(_filter_name: &str, ext: &str, file_name: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Some(PathBuf::from(home).join(format!("{file_name}.{ext}")))
}

#[cfg(windows)]
fn open_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("RoomForge plan", &["roomforge"])
        .pick_file()
}

#[cfg(not(windows))]
fn open_dialog() -> Option<PathBuf> {
    None
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Family theme: near-black panels, red accent.
        let mut style = (*ctx.style()).clone();
        style.visuals.panel_fill = PANEL_BG;
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(0x6B, 0x07, 0x00);
        style.visuals.selection.bg_fill = Color32::from_rgb(0x6B, 0x07, 0x00);
        ctx.set_style(style);

        self.handle_keys(ctx);
        self.top_bar(ctx);
        self.bottom_bar(ctx);
        if self.view != View::List {
            self.catalog_panel(ctx);
            self.properties_panel(ctx);
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Plan => self.plan_view(ui),
            View::ThreeD => self.three_d_view(ui),
            View::List => self.list_view(ui),
        });

        // Autosave at most every 5 seconds while there are changes.
        if self.dirty && self.last_autosave.elapsed().as_secs() >= 5 {
            let _ = roomforge::io::save_autosave(&self.project);
            self.last_autosave = Instant::now();
        }
    }
}

impl App {
    fn handle_keys(&mut self, ctx: &egui::Context) {
        // Don't steal keys while the user is typing in a text field.
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
            if ctrl && i.key_pressed(egui::Key::Z) {
                self.do_undo();
            } else if ctrl && i.key_pressed(egui::Key::Y) {
                self.do_redo();
            } else if ctrl && i.key_pressed(egui::Key::S) {
                if let Some(p) = self.current_path.clone() {
                    self.save_to(p);
                } else if let Some(p) = save_dialog("RoomForge plan", "roomforge", "my-room") {
                    self.save_to(p);
                }
            } else if i.key_pressed(egui::Key::Num1) {
                self.view = View::Plan;
            } else if i.key_pressed(egui::Key::Num2) {
                self.view = View::ThreeD;
            } else if i.key_pressed(egui::Key::Num3) {
                self.view = View::List;
            } else if i.key_pressed(egui::Key::Escape) {
                self.placing = None;
                self.selected_item = None;
            }
        });
        if let Some(id) = self.selected_item {
            ctx.input(|i| {
                let step = if i.modifiers.shift { 0.25 } else { 0.05 };
                let (mut dx, mut dy) = (0.0, 0.0);
                if i.key_pressed(egui::Key::ArrowLeft) {
                    dx = -step;
                } else if i.key_pressed(egui::Key::ArrowRight) {
                    dx = step;
                } else if i.key_pressed(egui::Key::ArrowUp) {
                    dy = step;
                } else if i.key_pressed(egui::Key::ArrowDown) {
                    dy = -step;
                }
                if dx != 0.0 || dy != 0.0 {
                    if !self.drag_snapshot_done {
                        self.snapshot();
                        self.drag_snapshot_done = true;
                    }
                    if let Some(it) = self.project.items.iter_mut().find(|it| it.id == id) {
                        it.x_m += dx;
                        it.y_m += dy;
                        let room = self.project.room.clone();
                        it.clamp_inside(&room);
                    }
                    self.mark_dirty();
                } else {
                    self.drag_snapshot_done = false;
                }
                if i.key_pressed(egui::Key::R) {
                    self.snapshot();
                    if let Some(it) = self.project.items.iter_mut().find(|it| it.id == id) {
                        it.rotation_deg += if i.modifiers.shift { 90.0 } else { 15.0 };
                    }
                    self.mark_dirty();
                }
                if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                    self.snapshot();
                    self.project.items.retain(|it| it.id != id);
                    self.selected_item = None;
                    self.mark_dirty();
                }
            });
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("RoomForge");
                ui.separator();
                if ui.button("New").on_hover_text("Start a new, empty room").clicked() {
                    self.snapshot();
                    self.project = Project::default();
                    self.selected_item = None;
                    self.current_path = None;
                    self.status = "New empty room.".into();
                    self.mark_dirty();
                }
                if ui.button("Open…").on_hover_text("Open a saved .roomforge plan").clicked() {
                    if let Some(p) = open_dialog() {
                        self.open_from(p);
                    } else {
                        self.status = "Open is available through the file dialog on Windows.".into();
                    }
                }
                if ui.button("Save").on_hover_text("Save the plan (Ctrl+S)").clicked() {
                    if let Some(p) = self.current_path.clone() {
                        self.save_to(p);
                    } else if let Some(p) = save_dialog("RoomForge plan", "roomforge", "my-room") {
                        self.save_to(p);
                    }
                }
                if ui.button("Save as…").on_hover_text("Save the plan under a new name — this is also how you share a plan: send the file").clicked() {
                    if let Some(p) = save_dialog("RoomForge plan", "roomforge", "my-room") {
                        self.save_to(p);
                    }
                }
                ui.separator();
                if ui.button("↩ Undo").on_hover_text("Undo (Ctrl+Z)").clicked() {
                    self.do_undo();
                }
                if ui.button("↪ Redo").on_hover_text("Redo (Ctrl+Y)").clicked() {
                    self.do_redo();
                }
                ui.separator();
                ui.selectable_value(&mut self.view, View::Plan, "2D plan")
                    .on_hover_text("Top-down plan (press 1)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D view")
                    .on_hover_text("Orbit the room in 3D (press 2)");
                ui.selectable_value(&mut self.view, View::List, "Item list")
                    .on_hover_text("Everything placed, grouped (press 3)");
                ui.separator();
                ui.checkbox(&mut self.snap_enabled, "Snap 5 cm")
                    .on_hover_text("Snap furniture to a 5 cm grid while dragging");
            });
            ui.horizontal(|ui| {
                ui.label("Export:");
                if ui.button("Plan (SVG)").on_hover_text("Save a to-scale drawing of the plan").clicked() {
                    if let Some(p) = save_dialog("SVG drawing", "svg", "plan") {
                        let svg = roomforge::export::plan_svg(&self.project, 110.0);
                        self.export_to(p, &svg, "Plan");
                    }
                }
                if ui.button("Item list (CSV)").on_hover_text("Save the grouped item list as a spreadsheet file").clicked() {
                    if let Some(p) = save_dialog("CSV list", "csv", "item-list") {
                        let csv = roomforge::export::list_csv(&self.project);
                        self.export_to(p, &csv, "Item list");
                    }
                }
                if ui.button("Print sheet (HTML)").on_hover_text("Save a printable page with the plan and item list — open it in a browser and print, or print to PDF").clicked() {
                    if let Some(p) = save_dialog("HTML print sheet", "html", "plan-print") {
                        let html = roomforge::export::print_html(&self.project);
                        self.export_to(p, &html, "Print sheet");
                    }
                }
            });
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        let warnings = self.project.warnings();
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{} · {:.2} m × {:.2} m · {:.1} m² · {} items",
                    self.project.room.name,
                    self.project.room.width_m,
                    self.project.room.depth_m,
                    self.project.room.area_m2(),
                    self.project.items.len()
                ));
                ui.separator();
                if warnings.is_empty() {
                    ui.label("No overlaps.");
                } else {
                    ui.colored_label(RED, format!("⚠ {}", warnings.join(" · ")));
                }
            });
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
        });
    }

    fn catalog_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("catalog")
            .default_width(215.0)
            .show(ctx, |ui| {
                ui.heading("Furniture");
                ui.label("Click an item, then click in the room to place it. Drag placed items to move them.");
                ui.text_edit_singleline(&mut self.catalog_search)
                    .on_hover_text("Filter the catalog by name");
                ui.separator();
                let search = self.catalog_search.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for cat in categories() {
                        let items: Vec<CatalogItem> = catalog()
                            .into_iter()
                            .filter(|c| c.category == cat && (search.is_empty() || c.name.to_lowercase().contains(&search)))
                            .collect();
                        if items.is_empty() {
                            continue;
                        }
                        egui::CollapsingHeader::new(cat).default_open(true).show(ui, |ui| {
                            for c in items {
                                let label = format!("{}  ({:.2}×{:.2} m)", c.name, c.width_m, c.depth_m);
                                let selected = self.placing.as_ref().map(|p| p.id) == Some(c.id);
                                if ui.selectable_label(selected, label).on_hover_text("Click, then click in the room to place it. Hold Shift while placing to place several.").clicked() {
                                    self.status = format!("Placing: {}. Click in the room.", c.name);
                                    self.placing = Some(c);
                                }
                            }
                        });
                    }
                });
            });
    }

    fn properties_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("props")
            .default_width(265.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.room_section(ui);
                    ui.separator();
                    self.wall_section(ui);
                    ui.separator();
                    self.selection_section(ui);
                });
            });
    }

    fn room_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Room");
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Name");
            if ui
                .text_edit_singleline(&mut self.project.room.name)
                .changed()
            {
                changed = true;
            }
        });
        macro_rules! dim {
            ($label:expr, $field:ident, $tip:expr) => {
                ui.horizontal(|ui| {
                    ui.label($label);
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.project.room.$field)
                                .speed(0.05)
                                .suffix(" m"),
                        )
                        .on_hover_text($tip)
                        .changed()
                    {
                        changed = true;
                    }
                });
            };
        }
        dim!(
            "Width (W–E)",
            width_m,
            "Room width, west to east, in metres"
        );
        dim!(
            "Depth (S–N)",
            depth_m,
            "Room depth, south to north, in metres"
        );
        dim!("Ceiling", height_m, "Ceiling height in metres");
        ui.horizontal(|ui| {
            ui.label("Wall colour");
            if ui
                .color_edit_button_srgb(&mut self.project.room.wall_rgb)
                .changed()
            {
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Floor colour");
            if ui
                .color_edit_button_srgb(&mut self.project.room.floor_rgb)
                .changed()
            {
                changed = true;
            }
        });
        if changed {
            self.project.room.clamp_dims();
            let room = self.project.room.clone();
            for o in &mut self.project.openings {
                o.clamp_to_wall(&room);
            }
            for s in &mut self.project.services {
                s.clamp_to_wall(&room);
            }
            for it in &mut self.project.items {
                it.clamp_inside(&room);
            }
            self.mark_dirty();
        }
    }

    fn wall_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Doors, windows & services");
        egui::ComboBox::from_label("On wall")
            .selected_text(self.wall_choice.label())
            .show_ui(ui, |ui| {
                for w in [Wall::North, Wall::South, Wall::East, Wall::West] {
                    ui.selectable_value(&mut self.wall_choice, w, w.label());
                }
            });
        ui.horizontal(|ui| {
            if ui
                .button("＋ Door")
                .on_hover_text("Add a door to the chosen wall")
                .clicked()
            {
                self.snapshot();
                let id = self.project.alloc_id();
                let len = self.wall_choice.length(&self.project.room);
                self.project.openings.push(Opening {
                    id,
                    kind: OpeningKind::Door,
                    wall: self.wall_choice,
                    offset_m: (len / 2.0 - 0.45).max(0.0),
                    width_m: 0.9,
                    sill_m: 0.0,
                    height_m: 2.1,
                    swings_in: true,
                });
                self.selected_opening = Some(id);
                self.mark_dirty();
            }
            if ui
                .button("＋ Window")
                .on_hover_text("Add a window to the chosen wall")
                .clicked()
            {
                self.snapshot();
                let id = self.project.alloc_id();
                let len = self.wall_choice.length(&self.project.room);
                self.project.openings.push(Opening {
                    id,
                    kind: OpeningKind::Window,
                    wall: self.wall_choice,
                    offset_m: (len / 2.0 - 0.75).max(0.0),
                    width_m: 1.5,
                    sill_m: 0.9,
                    height_m: 1.2,
                    swings_in: false,
                });
                self.selected_opening = Some(id);
                self.mark_dirty();
            }
        });
        ui.horizontal(|ui| {
            for (kind, label) in [
                (ServiceKind::Socket, "＋ Socket"),
                (ServiceKind::Pipe, "＋ Pipe"),
                (ServiceKind::Radiator, "＋ Radiator"),
            ] {
                if ui
                    .button(label)
                    .on_hover_text("Add this service point to the chosen wall")
                    .clicked()
                {
                    self.snapshot();
                    let id = self.project.alloc_id();
                    let len = self.wall_choice.length(&self.project.room);
                    self.project.services.push(ServicePoint {
                        id,
                        kind,
                        wall: self.wall_choice,
                        offset_m: len / 2.0,
                    });
                    self.selected_service = Some(id);
                    self.mark_dirty();
                }
            }
        });
        // Editors for the currently selected opening / service.
        if let Some(oid) = self.selected_opening {
            if let Some(o) = self.project.openings.iter_mut().find(|o| o.id == oid) {
                ui.separator();
                ui.label(format!(
                    "Selected: {} on {} wall",
                    o.kind.label(),
                    o.wall.label()
                ));
                let mut del = false;
                ui.horizontal(|ui| {
                    ui.label("From corner");
                    ui.add(
                        egui::DragValue::new(&mut o.offset_m)
                            .speed(0.05)
                            .suffix(" m"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Width");
                    ui.add(
                        egui::DragValue::new(&mut o.width_m)
                            .speed(0.05)
                            .suffix(" m"),
                    );
                });
                if o.kind == OpeningKind::Window {
                    ui.horizontal(|ui| {
                        ui.label("Sill height");
                        ui.add(egui::DragValue::new(&mut o.sill_m).speed(0.05).suffix(" m"));
                    });
                }
                if o.kind == OpeningKind::Door {
                    ui.checkbox(&mut o.swings_in, "Swings into the room");
                }
                let room = self.project.room.clone();
                o.clamp_to_wall(&room);
                if ui.button("Delete opening").clicked() {
                    del = true;
                }
                if del {
                    self.snapshot();
                    self.project.openings.retain(|x| x.id != oid);
                    self.selected_opening = None;
                    self.mark_dirty();
                } else {
                    self.mark_dirty();
                }
            }
        }
        if let Some(sid) = self.selected_service {
            if let Some(s) = self.project.services.iter_mut().find(|s| s.id == sid) {
                ui.separator();
                ui.label(format!(
                    "Selected: {} on {} wall",
                    s.kind.label(),
                    s.wall.label()
                ));
                ui.horizontal(|ui| {
                    ui.label("From corner");
                    ui.add(
                        egui::DragValue::new(&mut s.offset_m)
                            .speed(0.05)
                            .suffix(" m"),
                    );
                });
                let room = self.project.room.clone();
                s.clamp_to_wall(&room);
                if ui.button("Delete service point").clicked() {
                    self.snapshot();
                    self.project.services.retain(|x| x.id != sid);
                    self.selected_service = None;
                    self.mark_dirty();
                } else {
                    self.mark_dirty();
                }
            }
        }
        ui.label("Tip: click a door, window or service marker in the plan to edit it here.");
    }

    fn selection_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Selected furniture");
        let Some(id) = self.selected_item else {
            ui.label("Nothing selected. Click an item in the plan.");
            return;
        };
        let mut delete = false;
        let mut duplicate = false;
        if let Some(it) = self.project.items.iter_mut().find(|i| i.id == id) {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut it.name);
            });
            macro_rules! fdim {
                ($label:expr, $field:ident) => {
                    ui.horizontal(|ui| {
                        ui.label($label);
                        ui.add(
                            egui::DragValue::new(&mut it.$field)
                                .speed(0.02)
                                .suffix(" m"),
                        );
                    });
                };
            }
            fdim!("Centre X", x_m);
            fdim!("Centre Y", y_m);
            fdim!("Width", width_m);
            fdim!("Depth", depth_m);
            fdim!("Height", height_m);
            ui.horizontal(|ui| {
                ui.label("Rotation");
                ui.add(
                    egui::DragValue::new(&mut it.rotation_deg)
                        .speed(1.0)
                        .suffix("°"),
                )
                .on_hover_text("Rotation in degrees. R rotates by 15°, Shift+R by 90°");
            });
            ui.horizontal(|ui| {
                if ui.button("Rotate 90°").clicked() {
                    it.rotation_deg += 90.0;
                }
                ui.label("Colour");
                ui.color_edit_button_srgb(&mut it.rgb);
            });
            ui.horizontal(|ui| {
                if ui
                    .button("Duplicate")
                    .on_hover_text("Place a copy next to this item")
                    .clicked()
                {
                    duplicate = true;
                }
                if ui
                    .button("Delete")
                    .on_hover_text("Remove this item (Delete key)")
                    .clicked()
                {
                    delete = true;
                }
            });
            self.mark_dirty();
        }
        if duplicate {
            self.snapshot();
            if let Some(orig) = self.project.items.iter().find(|i| i.id == id).cloned() {
                let nid = self.project.alloc_id();
                let mut copy = orig;
                copy.id = nid;
                copy.x_m = (copy.x_m + 0.3).min(self.project.room.width_m);
                copy.y_m = (copy.y_m + 0.3).min(self.project.room.depth_m);
                self.project.items.push(copy);
                self.selected_item = Some(nid);
            }
            self.mark_dirty();
        }
        if delete {
            self.snapshot();
            self.project.items.retain(|i| i.id != id);
            self.selected_item = None;
            self.mark_dirty();
        }
    }

    // ---------- 2D plan ----------

    fn plan_view(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(rect, ui.id().with("plan"), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(0x08, 0x08, 0x08));

        let room = self.project.room.clone();
        let margin = 46.0f32;
        let scale = ((rect.width() - margin * 2.0) / room.width_m as f32)
            .min((rect.height() - margin * 2.0) / room.depth_m as f32)
            .max(5.0);
        let origin = Pos2::new(
            rect.left() + (rect.width() - room.width_m as f32 * scale) / 2.0,
            rect.bottom() - (rect.height() - room.depth_m as f32 * scale) / 2.0,
        );
        // Room coords (metres, y north) → screen.
        let to_screen =
            |x: f64, y: f64| Pos2::new(origin.x + x as f32 * scale, origin.y - y as f32 * scale);
        let to_room = |pos: Pos2| {
            (
                ((pos.x - origin.x) / scale) as f64,
                ((origin.y - pos.y) / scale) as f64,
            )
        };

        // Grid every 0.5 m.
        let grid_col = Color32::from_rgba_unmultiplied(255, 255, 255, 22);
        let mut gx = 0.0;
        while gx <= room.width_m + 1e-9 {
            painter.line_segment(
                [to_screen(gx, 0.0), to_screen(gx, room.depth_m)],
                Stroke::new(1.0_f32, grid_col),
            );
            gx += 0.5;
        }
        let mut gy = 0.0;
        while gy <= room.depth_m + 1e-9 {
            painter.line_segment(
                [to_screen(0.0, gy), to_screen(room.width_m, gy)],
                Stroke::new(1.0_f32, grid_col),
            );
            gy += 0.5;
        }
        // Floor + walls.
        painter.rect_filled(
            Rect::from_two_pos(to_screen(0.0, room.depth_m), to_screen(room.width_m, 0.0)),
            0.0,
            rgb(room.floor_rgb),
        );
        painter.rect_stroke(
            Rect::from_two_pos(to_screen(0.0, room.depth_m), to_screen(room.width_m, 0.0)),
            0.0,
            Stroke::new(4.0_f32, Color32::from_rgb(230, 230, 230)),
            egui::StrokeKind::Middle,
        );
        // Dimension labels.
        let font = FontId::proportional(13.0);
        painter.text(
            Pos2::new(
                origin.x + room.width_m as f32 * scale / 2.0,
                origin.y + 18.0,
            ),
            Align2::CENTER_TOP,
            format!("{:.2} m", room.width_m),
            font.clone(),
            Color32::LIGHT_GRAY,
        );
        painter.text(
            Pos2::new(
                origin.x - 10.0,
                origin.y - room.depth_m as f32 * scale / 2.0,
            ),
            Align2::RIGHT_CENTER,
            format!("{:.2} m", room.depth_m),
            font,
            Color32::LIGHT_GRAY,
        );

        self.draw_openings_2d(&painter, &to_screen);
        self.draw_services_2d(&painter, &to_screen);

        // Furniture.
        for it in &self.project.items {
            let pts: Vec<Pos2> = it.corners().iter().map(|&(x, y)| to_screen(x, y)).collect();
            let selected = self.selected_item == Some(it.id);
            painter.add(Shape::convex_polygon(
                pts.clone(),
                rgb(it.rgb),
                Stroke::new(
                    if selected { 3.0_f32 } else { 1.5_f32 },
                    if selected {
                        RED
                    } else {
                        Color32::from_rgb(20, 20, 20)
                    },
                ),
            ));
            let centre = to_screen(it.x_m, it.y_m);
            let w_px = (it.width_m as f32 * scale).abs();
            if w_px > 52.0 {
                painter.text(
                    centre,
                    Align2::CENTER_CENTER,
                    &it.name,
                    FontId::proportional(11.0),
                    Color32::from_rgb(15, 15, 15),
                );
            }
        }

        // Placement ghost / interactions.
        if let Some(pos) = response.interact_pointer_pos() {
            let (rx, ry) = to_room(pos);
            if let Some(cat) = self.placing.clone() {
                // Ghost preview.
                let fake = PlacedItem {
                    id: 0,
                    catalog_id: cat.id.into(),
                    name: cat.name.into(),
                    category: cat.category.into(),
                    x_m: rx,
                    y_m: ry,
                    width_m: cat.width_m,
                    depth_m: cat.depth_m,
                    height_m: cat.height_m,
                    rotation_deg: 0.0,
                    rgb: cat.rgb,
                };
                let pts: Vec<Pos2> = fake
                    .corners()
                    .iter()
                    .map(|&(x, y)| to_screen(x, y))
                    .collect();
                painter.add(Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(229, 72, 77, 110),
                    Stroke::new(2.0_f32, RED),
                ));
                if response.clicked() {
                    self.snapshot();
                    let id = self.project.alloc_id();
                    let mut item = fake;
                    item.id = id;
                    if self.snap_enabled {
                        item.x_m = snap(item.x_m, 0.05);
                        item.y_m = snap(item.y_m, 0.05);
                    }
                    item.clamp_inside(&self.project.room.clone());
                    self.project.items.push(item);
                    self.selected_item = Some(id);
                    self.mark_dirty();
                    let keep = ui.input(|i| i.modifiers.shift);
                    if !keep {
                        self.placing = None;
                    }
                }
                return;
            }

            if response.drag_started() {
                // Topmost item under the pointer wins.
                let hit = self
                    .project
                    .items
                    .iter()
                    .rev()
                    .find(|it| point_in_obb(rx, ry, &it.corners()))
                    .map(|it| it.id);
                if let Some(id) = hit {
                    self.selected_item = Some(id);
                    self.selected_opening = None;
                    self.selected_service = None;
                    let it = self.project.items.iter().find(|i| i.id == id).unwrap();
                    self.drag_offset = (it.x_m - rx, it.y_m - ry);
                    self.drag_item = Some(id);
                    self.drag_snapshot_done = false;
                } else {
                    // Maybe an opening or a service marker?
                    self.select_fitting_at(rx, ry);
                }
            }
            if response.dragged() {
                if let Some(id) = self.drag_item {
                    if !self.drag_snapshot_done {
                        self.snapshot();
                        self.drag_snapshot_done = true;
                    }
                    let (ox, oy) = self.drag_offset;
                    if let Some(it) = self.project.items.iter_mut().find(|i| i.id == id) {
                        it.x_m = rx + ox;
                        it.y_m = ry + oy;
                        if self.snap_enabled {
                            it.x_m = snap(it.x_m, 0.05);
                            it.y_m = snap(it.y_m, 0.05);
                        }
                        let room2 = self.project.room.clone();
                        it.clamp_inside(&room2);
                    }
                    self.mark_dirty();
                }
            }
            if response.drag_stopped() {
                self.drag_item = None;
                self.drag_snapshot_done = false;
            }
            if response.clicked() && self.drag_item.is_none() {
                // Click on empty space clears the furniture selection.
                let hit = self
                    .project
                    .items
                    .iter()
                    .rev()
                    .find(|it| point_in_obb(rx, ry, &it.corners()))
                    .map(|it| it.id);
                if hit.is_none() {
                    self.selected_item = None;
                    self.select_fitting_at(rx, ry);
                } else {
                    self.selected_item = hit;
                }
            }
        }
        let _ = Vec2::ZERO; // keep Vec2 import used on all targets
    }

    fn select_fitting_at(&mut self, rx: f64, ry: f64) {
        // Services: within 0.25 m of their centre.
        for s in &self.project.services {
            let (cx, cy) = s.centre(&self.project.room);
            if (cx - rx).abs() < 0.25 && (cy - ry).abs() < 0.25 {
                self.selected_service = Some(s.id);
                self.selected_opening = None;
                return;
            }
        }
        // Openings: near the wall, within the opening's span.
        for o in &self.project.openings {
            let near = match o.wall {
                Wall::South => ry.abs() < 0.3 && rx >= o.offset_m && rx <= o.offset_m + o.width_m,
                Wall::North => {
                    (ry - self.project.room.depth_m).abs() < 0.3
                        && rx >= o.offset_m
                        && rx <= o.offset_m + o.width_m
                }
                Wall::West => rx.abs() < 0.3 && ry >= o.offset_m && ry <= o.offset_m + o.width_m,
                Wall::East => {
                    (rx - self.project.room.width_m).abs() < 0.3
                        && ry >= o.offset_m
                        && ry <= o.offset_m + o.width_m
                }
            };
            if near {
                self.selected_opening = Some(o.id);
                self.selected_service = None;
                return;
            }
        }
        self.selected_opening = None;
        self.selected_service = None;
    }

    fn draw_openings_2d(&self, painter: &egui::Painter, to_screen: &dyn Fn(f64, f64) -> Pos2) {
        for o in &self.project.openings {
            let (w, d) = (self.project.room.width_m, self.project.room.depth_m);
            let (a, b) = match o.wall {
                Wall::South => ((o.offset_m, 0.0), (o.offset_m + o.width_m, 0.0)),
                Wall::North => ((o.offset_m, d), (o.offset_m + o.width_m, d)),
                Wall::West => ((0.0, o.offset_m), (0.0, o.offset_m + o.width_m)),
                Wall::East => ((w, o.offset_m), (w, o.offset_m + o.width_m)),
            };
            let selected = self.selected_opening == Some(o.id);
            match o.kind {
                OpeningKind::Window => {
                    painter.line_segment(
                        [to_screen(a.0, a.1), to_screen(b.0, b.1)],
                        Stroke::new(7.0_f32, Color32::from_rgb(80, 160, 230)),
                    );
                }
                OpeningKind::Door => {
                    painter.line_segment(
                        [to_screen(a.0, a.1), to_screen(b.0, b.1)],
                        Stroke::new(5.0_f32, rgb(self.project.room.floor_rgb)),
                    );
                    if o.swings_in {
                        let inward = match o.wall {
                            Wall::South => (a.0, a.1 + o.width_m),
                            Wall::North => (a.0, a.1 - o.width_m),
                            Wall::West => (a.0 + o.width_m, a.1),
                            Wall::East => (a.0 - o.width_m, a.1),
                        };
                        painter.line_segment(
                            [to_screen(a.0, a.1), to_screen(inward.0, inward.1)],
                            Stroke::new(2.0_f32, Color32::from_rgb(230, 230, 230)),
                        );
                    }
                }
            }
            if selected {
                painter.circle_stroke(to_screen(a.0, a.1), 7.0, Stroke::new(2.0_f32, RED));
                painter.circle_stroke(to_screen(b.0, b.1), 7.0, Stroke::new(2.0_f32, RED));
            }
        }
    }

    fn draw_services_2d(&self, painter: &egui::Painter, to_screen: &dyn Fn(f64, f64) -> Pos2) {
        for s in &self.project.services {
            let (cx, cy) = s.centre(&self.project.room);
            let (label, fill) = match s.kind {
                ServiceKind::Socket => ("E", Color32::from_rgb(246, 195, 67)),
                ServiceKind::Pipe => ("W", Color32::from_rgb(59, 130, 196)),
                ServiceKind::Radiator => ("R", Color32::from_rgb(196, 59, 59)),
            };
            let pos = to_screen(cx, cy);
            let selected = self.selected_service == Some(s.id);
            painter.circle_filled(pos, 10.0, fill);
            if selected {
                painter.circle_stroke(pos, 13.0, Stroke::new(2.0_f32, RED));
            }
            painter.text(
                pos,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(11.0),
                Color32::from_rgb(10, 10, 10),
            );
        }
    }

    // ---------- 3D view ----------

    fn three_d_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Orbit");
            ui.add(egui::Slider::new(&mut self.camera.yaw_deg, 0.0..=360.0).text("Yaw°"));
            ui.add(egui::Slider::new(&mut self.camera.pitch_deg, 15.0..=80.0).text("Pitch°"));
            ui.add(egui::Slider::new(&mut self.camera.scale, 25.0..=160.0).text("Zoom"));
            ui.label("Drag the view to orbit.");
        });
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(rect, ui.id().with("3d"), Sense::drag());
        if response.dragged() {
            let d = response.drag_delta();
            self.camera.yaw_deg = (self.camera.yaw_deg + d.x as f64 * 0.4) % 360.0;
            self.camera.pitch_deg = (self.camera.pitch_deg + d.y as f64 * 0.3).clamp(15.0, 80.0);
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(0x08, 0x08, 0x08));
        let room = &self.project.room;
        let centre = (room.width_m / 2.0, room.depth_m / 2.0);
        let origin = (
            rect.center().x as f64,
            (rect.center().y + rect.height() * 0.18) as f64,
        );
        let cam = self.camera;
        let pr = |p: (f64, f64, f64)| {
            let (sx, sy, depth) = project3d(p, centre, &cam, origin);
            (Pos2::new(sx as f32, sy as f32), depth)
        };

        // Gather faces: (points, colour, avg depth). Painted far → near.
        let mut faces: Vec<(Vec<Pos2>, Color32, f64)> = Vec::new();
        // Floor
        {
            let corners = [
                (0.0, 0.0, 0.0),
                (room.width_m, 0.0, 0.0),
                (room.width_m, room.depth_m, 0.0),
                (0.0, room.depth_m, 0.0),
            ];
            let (pts, depth) = face_of(&corners, &pr);
            faces.push((pts, rgb(shade(room.floor_rgb, 0.9)), depth));
        }
        // Walls (quads up to ceiling height), with window/door gaps drawn
        // as coloured insets afterwards.
        type WallQuad = ([(f64, f64, f64); 4], f64);
        let wall_quads: [WallQuad; 4] = [
            (
                [
                    (0.0, 0.0, 0.0),
                    (room.width_m, 0.0, 0.0),
                    (room.width_m, 0.0, room.height_m),
                    (0.0, 0.0, room.height_m),
                ],
                0.75,
            ),
            (
                [
                    (0.0, room.depth_m, 0.0),
                    (room.width_m, room.depth_m, 0.0),
                    (room.width_m, room.depth_m, room.height_m),
                    (0.0, room.depth_m, room.height_m),
                ],
                0.95,
            ),
            (
                [
                    (0.0, 0.0, 0.0),
                    (0.0, room.depth_m, 0.0),
                    (0.0, room.depth_m, room.height_m),
                    (0.0, 0.0, room.height_m),
                ],
                0.85,
            ),
            (
                [
                    (room.width_m, 0.0, 0.0),
                    (room.width_m, room.depth_m, 0.0),
                    (room.width_m, room.depth_m, room.height_m),
                    (room.width_m, 0.0, room.height_m),
                ],
                0.65,
            ),
        ];
        for (quad, bright) in wall_quads {
            let (pts, depth) = face_of(&quad, &pr);
            faces.push((pts, rgb(shade(room.wall_rgb, bright)), depth));
        }
        // Furniture boxes
        for it in &self.project.items {
            let corners = box_corners(it);
            for (idx, bright) in box_faces() {
                let quad = [
                    corners[idx[0]],
                    corners[idx[1]],
                    corners[idx[2]],
                    corners[idx[3]],
                ];
                let (pts, depth) = face_of(&quad, &pr);
                let col = if self.selected_item == Some(it.id) && bright == 1.0 {
                    RED
                } else {
                    rgb(shade(it.rgb, bright))
                };
                faces.push((pts, col, depth));
            }
        }
        faces.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        for (pts, col, _) in faces {
            if pts.len() >= 3 {
                painter.add(Shape::convex_polygon(
                    pts,
                    col,
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 60)),
                ));
            }
        }
        // Windows glow on top of the walls they sit in.
        for o in &self.project.openings {
            if o.kind != OpeningKind::Window {
                continue;
            }
            let quad = wall_opening_quad(&self.project, o);
            let (pts, _) = face_of(&quad, &pr);
            painter.add(Shape::convex_polygon(
                pts,
                Color32::from_rgba_unmultiplied(120, 190, 240, 200),
                Stroke::NONE,
            ));
        }
        painter.text(
            Pos2::new(rect.left() + 10.0, rect.bottom() - 12.0),
            Align2::LEFT_BOTTOM,
            format!("{} — 3D view", room.name),
            FontId::proportional(12.0),
            Color32::GRAY,
        );
    }

    // ---------- item list ----------

    fn list_view(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} — item list", self.project.room.name));
        ui.label(format!(
            "Room {:.2} m × {:.2} m, ceiling {:.2} m, floor area {:.1} m². No prices are tracked in RoomForge.",
            self.project.room.width_m,
            self.project.room.depth_m,
            self.project.room.height_m,
            self.project.room.area_m2()
        ));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("items")
                .striped(true)
                .num_columns(4)
                .show(ui, |ui| {
                    ui.strong("Item");
                    ui.strong("Category");
                    ui.strong("Size (W × D × H)");
                    ui.strong("Qty");
                    ui.end_row();
                    for (name, cat, w, d, h, n) in self.project.item_list() {
                        ui.label(name);
                        ui.label(cat);
                        ui.label(format!("{w:.2} × {d:.2} × {h:.2} m"));
                        ui.label(n.to_string());
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.strong("Doors, windows and services");
            for o in &self.project.openings {
                ui.label(format!(
                    "{} — {} wall, {:.2} m from the corner, {:.2} m wide",
                    o.kind.label(),
                    o.wall.label(),
                    o.offset_m,
                    o.width_m
                ));
            }
            for s in &self.project.services {
                ui.label(format!(
                    "{} — {} wall, {:.2} m from the corner",
                    s.kind.label(),
                    s.wall.label(),
                    s.offset_m
                ));
            }
            let warnings = self.project.warnings();
            if !warnings.is_empty() {
                ui.separator();
                ui.colored_label(RED, "Warnings:");
                for w in warnings {
                    ui.colored_label(RED, format!("⚠ {w}"));
                }
            }
        });
    }
}

/// Project a quad and return its points plus average depth.
fn face_of(
    quad: &[(f64, f64, f64); 4],
    pr: &dyn Fn((f64, f64, f64)) -> (Pos2, f64),
) -> (Vec<Pos2>, f64) {
    let mut pts = Vec::with_capacity(4);
    let mut depth = 0.0;
    for &p in quad {
        let (pos, d) = pr(p);
        pts.push(pos);
        depth += d;
    }
    (pts, depth / 4.0)
}

/// The 3D quad of a wall opening, slightly proud of the wall surface.
fn wall_opening_quad(p: &Project, o: &Opening) -> [(f64, f64, f64); 4] {
    let (w, d) = (p.room.width_m, p.room.depth_m);
    let (z0, z1) = (o.sill_m, o.sill_m + o.height_m);
    let e = 0.01;
    match o.wall {
        Wall::South => [
            (o.offset_m, -e, z0),
            (o.offset_m + o.width_m, -e, z0),
            (o.offset_m + o.width_m, -e, z1),
            (o.offset_m, -e, z1),
        ],
        Wall::North => [
            (o.offset_m, d + e, z0),
            (o.offset_m + o.width_m, d + e, z0),
            (o.offset_m + o.width_m, d + e, z1),
            (o.offset_m, d + e, z1),
        ],
        Wall::West => [
            (-e, o.offset_m, z0),
            (-e, o.offset_m + o.width_m, z0),
            (-e, o.offset_m + o.width_m, z1),
            (-e, o.offset_m, z1),
        ],
        Wall::East => [
            (w + e, o.offset_m, z0),
            (w + e, o.offset_m + o.width_m, z0),
            (w + e, o.offset_m + o.width_m, z1),
            (w + e, o.offset_m, z1),
        ],
    }
}
