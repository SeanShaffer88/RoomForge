//! RoomForge GUI. Renders state and collects intent only — every rule
//! (geometry, grouping, warnings, projection, export) lives in the
//! library. There are no prices anywhere in this UI, by design.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use roomforge::catalog::{catalog, categories, CatalogItem};
use roomforge::geometry::{align_snap, point_in_obb, snap};
use roomforge::model::{
    Annotation, Corner, Opening, OpeningKind, PlacedItem, Project, SavedView, ServiceKind,
    ServicePoint, Wall,
};
use roomforge::model3d::PreparedModel;
use roomforge::units::{fmt_area, fmt_len, parse_len};
use roomforge::view3d::{
    box_corners, box_faces, project as project3d, shade_scene, style_rgb, wall_opening_quad,
    Camera, Scene, Style, WallCut,
};
use roomforge::walk::{walk_layers, WalkCmd, Walker};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Plan,
    ThreeD,
    List,
}

/// One thing to paint in the 3D view, depth-sorted with everything
/// else (fills for faces, lines for floor pattern segments).
enum Draw {
    Fill(Vec<Pos2>, Color32),
    Line([Pos2; 2], Color32),
}

/// What is about to be placed: a built-in catalog piece or one of the
/// user's own custom definitions, in one owned shape.
#[derive(Clone)]
struct PlaceTemplate {
    catalog_id: String,
    name: String,
    category: String,
    width_m: f64,
    depth_m: f64,
    height_m: f64,
    rgb: [u8; 3],
}

impl From<&CatalogItem> for PlaceTemplate {
    fn from(c: &CatalogItem) -> Self {
        PlaceTemplate {
            catalog_id: c.id.to_string(),
            name: c.name.to_string(),
            category: c.category.to_string(),
            width_m: c.width_m,
            depth_m: c.depth_m,
            height_m: c.height_m,
            rgb: c.rgb,
        }
    }
}

impl From<&roomforge::model::CustomItemDef> for PlaceTemplate {
    fn from(d: &roomforge::model::CustomItemDef) -> Self {
        PlaceTemplate {
            catalog_id: d.catalog_id(),
            name: d.name.clone(),
            category: d.category.clone(),
            width_m: d.width_m,
            depth_m: d.depth_m,
            height_m: d.height_m,
            rgb: d.rgb,
        }
    }
}

pub struct App {
    project: Project,
    view: View,
    camera: Camera,
    selected_item: Option<u64>,
    selected_opening: Option<u64>,
    selected_service: Option<u64>,
    selected_note: Option<u64>,
    placing: Option<PlaceTemplate>,
    show_help: bool,
    show_gallery: bool,
    gallery_for_room: bool,
    text_scale: f32,
    scene: Scene,
    style: Style,
    wall_cut: WallCut,
    show_views: bool,
    view_name: String,
    walking: bool,
    walker: Walker,
    // Room-shape (notch) form state.
    notch_h: f64,
    notch_v: f64,
    // Custom-item builder form state.
    custom_name: String,
    model_cache: HashMap<u64, PreparedModel>,
    model_failed: HashSet<u64>,
    custom_category: String,
    custom_w: f64,
    custom_d: f64,
    custom_h: f64,
    custom_rgb: [u8; 3],
    placing_note: bool,
    imperial: bool,
    /// Decoded underlay texture, keyed by (byte length, w, h) of the
    /// embedded image so project changes re-decode automatically.
    underlay_tex: Option<((usize, u32, u32), egui::TextureHandle)>,
    calibrating: bool,
    calib_marks: Vec<(f64, f64)>,
    calib_dist_m: f64,
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
    catalog_dragging: bool,
    drag_guides: (Vec<f64>, Vec<f64>),
    drag_offset: (f64, f64),
    drag_snapshot_done: bool,
}

const RED: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
const PANEL_BG: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A length field that displays and accepts the user's chosen units
/// (metric: "2.5 m" / "250 cm"; imperial: 10' 6"). Storage stays in
/// metres. Returns true when a new value was committed.
fn len_edit(ui: &mut egui::Ui, key: &str, value: &mut f64, imperial: bool, tip: &str) -> bool {
    let wid = ui.id().with(key);
    let focused = ui.memory(|m| m.has_focus(wid));
    let mut text = ui.data_mut(|d| d.get_temp::<String>(wid).unwrap_or_default());
    if !focused {
        text = fmt_len(*value, imperial);
    }
    let resp = ui
        .add(
            egui::TextEdit::singleline(&mut text)
                .id(wid)
                .desired_width(92.0),
        )
        .on_hover_text(tip);
    let mut committed = false;
    if resp.has_focus() {
        ui.data_mut(|d| d.insert_temp(wid, text.clone()));
    }
    if resp.lost_focus() {
        if let Some(v) = parse_len(&text, imperial) {
            if (v - *value).abs() > 1e-9 {
                *value = v;
                committed = true;
            }
        }
        ui.data_mut(|d| d.insert_temp(wid, fmt_len(*value, imperial)));
    }
    committed
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
    add(&mut p, "floor-lamp", 0.32, 3.35, 0.0);
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
        links_to: None,
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
        links_to: None,
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

/// Place a catalog piece into a template project at (x, y).
fn tpl_place(p: &mut Project, catalog_id: &str, x: f64, y: f64, rot: f64) {
    if let Some(c) = catalog().into_iter().find(|c| c.id == catalog_id) {
        let id = p.alloc_id();
        let mut it = PlacedItem {
            id,
            catalog_id: c.id.into(),
            name: c.name.into(),
            category: c.category.into(),
            x_m: x,
            y_m: y,
            width_m: c.width_m,
            depth_m: c.depth_m,
            height_m: c.height_m,
            rotation_deg: rot,
            rgb: c.rgb,
        };
        it.clamp_inside(&p.room.clone());
        p.items.push(it);
    }
}

fn tpl_door_window(p: &mut Project) {
    let did = p.alloc_id();
    p.openings.push(Opening {
        id: did,
        kind: OpeningKind::Door,
        wall: Wall::South,
        offset_m: 0.4,
        width_m: 0.9,
        sill_m: 0.0,
        height_m: 2.1,
        swings_in: true,
        links_to: None,
    });
    let wid = p.alloc_id();
    p.openings.push(Opening {
        id: wid,
        kind: OpeningKind::Window,
        wall: Wall::North,
        offset_m: 1.2,
        width_m: 1.5,
        sill_m: 0.9,
        height_m: 1.2,
        swings_in: false,
        links_to: None,
    });
}

fn bedroom_project() -> Project {
    let mut p = Project::default();
    p.room.name = "Bedroom".into();
    p.room.width_m = 4.2;
    p.room.depth_m = 3.6;
    tpl_door_window(&mut p);
    tpl_place(&mut p, "bed-double", 1.35, 2.5, 0.0);
    tpl_place(&mut p, "bedside-table", 2.45, 3.25, 0.0);
    tpl_place(&mut p, "wardrobe-2", 3.5, 3.3, 0.0);
    tpl_place(&mut p, "chest-4", 3.85, 1.6, 270.0);
    tpl_place(&mut p, "rug-small", 1.6, 1.4, 0.0);
    tpl_place(&mut p, "floor-lamp", 0.32, 3.3, 0.0);
    p
}

fn office_project() -> Project {
    let mut p = Project::default();
    p.room.name = "Home office".into();
    p.room.width_m = 3.6;
    p.room.depth_m = 3.2;
    tpl_door_window(&mut p);
    tpl_place(&mut p, "desk", 1.8, 2.85, 0.0);
    tpl_place(&mut p, "desk-chair", 1.8, 2.1, 180.0);
    tpl_place(&mut p, "bookcase", 0.35, 1.6, 90.0);
    tpl_place(&mut p, "shelf-unit", 3.3, 1.2, 270.0);
    tpl_place(&mut p, "plant", 3.3, 2.9, 0.0);
    tpl_place(&mut p, "rug-small", 1.8, 1.5, 0.0);
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
                "Welcome to RoomForge. Pick furniture on the left, click in the room to place it. Press F1 or click Help (bottom right of the top bar) any time."
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
            selected_note: None,
            placing: None,
            show_help: false,
            show_gallery: false,
            gallery_for_room: false,
            text_scale: 1.0,
            scene: Scene::Day,
            style: Style::Coloured,
            wall_cut: WallCut::Full,
            show_views: false,
            view_name: String::new(),
            walking: false,
            walker: Walker::default(),
            notch_h: 1.5,
            notch_v: 1.2,
            custom_name: String::new(),
            model_cache: HashMap::new(),
            model_failed: HashSet::new(),
            custom_category: "Other".to_string(),
            custom_w: 0.6,
            custom_d: 0.45,
            custom_h: 0.8,
            custom_rgb: [120, 110, 100],
            placing_note: false,
            imperial: false,
            underlay_tex: None,
            calibrating: false,
            calib_marks: Vec::new(),
            calib_dist_m: 1.0,
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
            catalog_dragging: false,
            drag_guides: (Vec::new(), Vec::new()),
            drag_offset: (0.0, 0.0),
            drag_snapshot_done: false,
        }
    }

    /// Open a plan file passed on the command line.
    pub fn open_path(&mut self, path: PathBuf) {
        self.open_from(path);
    }

    /// Load a floor-plan image as the tracing underlay. The original
    /// bytes are embedded in the project so the plan stays portable.
    fn load_underlay(&mut self, path: PathBuf) {
        use base64::Engine;
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("Could not read the image: {e}");
                return;
            }
        };
        if bytes.len() > 12 * 1024 * 1024 {
            self.status = "That image is over 12 MB — please use a smaller scan or photo.".into();
            return;
        }
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i,
            Err(e) => {
                self.status = format!("That file could not be decoded as an image: {e}");
                return;
            }
        };
        self.snapshot();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        self.project.underlay = Some(roomforge::model::Underlay::fit(
            &self.project.room,
            img.width(),
            img.height(),
            b64,
        ));
        self.underlay_tex = None;
        self.status =
            "Plan image loaded under the drawing. Use Calibrate in the Room panel to set its true scale.".into();
        self.mark_dirty();
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

    fn save_snapshot(&mut self) {
        let name = format!(
            "{}-3d",
            self.project
                .room
                .name
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
        );
        if let Some(path) = save_dialog("PNG image", "png", &name) {
            let img = roomforge::render::render_snapshot_full(
                &self.project,
                1280,
                800,
                self.scene,
                self.style,
                self.wall_cut,
            );
            match img.save(&path) {
                Ok(()) => self.status = format!("3D snapshot written to {}", path.display()),
                Err(e) => self.status = format!("Could not write {}: {e}", path.display()),
            }
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

#[cfg(windows)]
fn open_image_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Plan image", &["png", "jpg", "jpeg", "bmp"])
        .pick_file()
}

#[cfg(not(windows))]
fn open_image_dialog() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn open_model_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("3D model", &["glb", "obj"])
        .pick_file()
}

#[cfg(not(windows))]
fn open_model_dialog() -> Option<PathBuf> {
    None
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
        ctx.set_zoom_factor(self.text_scale);

        self.handle_keys(ctx);
        if self.show_help {
            self.help_window(ctx);
        }
        if self.show_gallery {
            self.gallery_window(ctx);
        }
        if self.show_views {
            self.views_window(ctx);
        }
        self.sync_model_cache();
        self.top_bar(ctx);
        self.room_tabs(ctx);
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

        // A catalog drag that ended anywhere else just disarms.
        if self.catalog_dragging && ctx.input(|i| !i.pointer.any_down()) {
            self.catalog_dragging = false;
        }

        // Autosave at most every 5 seconds while there are changes.
        if self.dirty && self.last_autosave.elapsed().as_secs() >= 5 {
            let _ = roomforge::io::save_autosave(&self.project);
            self.last_autosave = Instant::now();
        }
    }
}

impl App {
    fn handle_keys(&mut self, ctx: &egui::Context) {
        // F1 opens Help even while typing somewhere.
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.show_help = true;
        }
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
                self.walking = false;
            } else if i.key_pressed(egui::Key::Num2) {
                self.view = View::ThreeD;
            } else if i.key_pressed(egui::Key::Num3) {
                self.view = View::List;
                self.walking = false;
            } else if i.key_pressed(egui::Key::Escape) {
                self.placing = None;
                self.placing_note = false;
                self.calibrating = false;
                self.calib_marks.clear();
                self.selected_item = None;
                self.selected_note = None;
            } else if (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
                && self.selected_item.is_none()
            {
                if let Some(nid) = self.selected_note {
                    self.snapshot();
                    self.project.annotations.retain(|a| a.id != nid);
                    self.selected_note = None;
                    self.mark_dirty();
                }
            }
        });
        if let Some(id) = self.selected_item.filter(|_| !self.walking) {
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

    /// The plain-language guide. Everything a first-time user needs,
    /// in words that assume no design background.
    fn help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("Help — RoomForge in plain language")
            .open(&mut open)
            .default_width(540.0)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Start in four steps");
                    ui.label("1. Set your room's size in the Room panel on the right. Width runs left-to-right; depth runs front-to-back.");
                    ui.label("2. Add doors and windows in “Doors, windows & services”: choose a wall, then tick Door or Window.");
                    ui.label("3. Click a piece of furniture on the left, then click in the room to place it — or drag it straight in from the list. Drag pieces to move them: they snap to the grid and align magnetically with each other, with red guide lines and the gaps to each wall shown as you go.");
                    ui.label("4. Check the result in the 3D view, and use the Item list to see everything's size and the materials quantities.");
                    ui.separator();
                    ui.heading("Words we use");
                    ui.label("• Services — the fixed points on your walls: power sockets, water pipes, radiators, light switches, TV and network points, thermostats.");
                    ui.label("• Notch — a rectangular bite cut out of a corner of the room, which is how you make an L-shaped room.");
                    ui.label("• Tracing image — a photo or scan of a floor plan laid under your drawing so you can trace over it (Room panel, “Blueprint tracing”).");
                    ui.label("• Materials list — how much floor, paint and skirting the room needs, as areas and lengths. RoomForge never tracks prices, by design.");
                    ui.separator();
                    ui.heading("Mouse and keys");
                    egui::Grid::new("help-keys").striped(true).num_columns(2).show(ui, |ui| {
                        let rows = [
                            ("Click a list item, then click the room", "Place that furniture"),
                            ("Drag a placed item", "Move it (5 cm steps with snapping on)"),
                            ("Arrow keys", "Nudge the selected item by 5 cm"),
                            ("Shift + arrow keys", "Nudge by 25 cm"),
                            ("R / Shift+R", "Turn the selected item by 15° / 90°"),
                            ("Delete", "Remove the selected item or note"),
                            ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
                            ("Ctrl+S", "Save the plan"),
                            ("1 / 2 / 3", "Switch to 2D plan / 3D view / Item list"),
                            ("Esc", "Stop placing, or clear the selection"),
                            ("F1", "Open this help"),
                        ];
                        for (keys, what) in rows {
                            ui.strong(keys);
                            ui.label(what);
                            ui.end_row();
                        }
                    });
                    ui.separator();
                    ui.heading("Good to know");
                    ui.label("• RoomForge works completely offline. Your plans never leave your computer, and there are no accounts.");
                    ui.label("• Your work saves itself as you go: if the program closes unexpectedly, your last session is restored next time you start it.");
                    ui.label("• A plan is a single file ending in .roomforge — copy it, email it, or back it up like any document.");
                    ui.label("• Text too small? Use the A− / A+ buttons at the right of the Export row to change the size of everything.");
                    ui.label("• In the 3D view, “Walk through the room” steps inside it: W A S D move, Q/E or dragging turns your head, Esc steps back out. You bump into walls and furniture, just like real life — and “Save PNG of this view” keeps a picture of where you stood.");
                    ui.label("• New → Template gallery shows the starter rooms as pictures, if you would rather rearrange than start from bare walls.");
                    ui.label("• A plan can hold several rooms: the Rooms strip under the top bar switches between them, “+ Add room” adds another (each room keeps its own size, flooring and furniture), and a door can be told which room it leads to. The item list and print sheet cover the whole home.");
                    ui.label("• Got a 3D model of your own furniture (a .glb or .obj file)? “Your 3D models” in the catalogue imports it — it renders with its real shape in 3D and on walks, and travels inside the plan file.");
                    ui.label("• Presenting a room: the 3D view's Style (Coloured/Clay) and Walls (Full/Half/Low) rows change how it looks, and “Saved views…” stores a whole setup — camera, lighting, style and wall cut — in the plan to jump back to.");
                    ui.label("• Pros: every measurement accepts typed values in either unit system (try “10' 6\\\"” or “2.4 m”), exports include true-scale 1:50 and 1:100 print plans, and the 3D view's “Save PNG snapshot” renders a shareable picture of the room.");
                });
            });
        self.show_help = open;
    }

    /// Make a room active in the UI: the library swaps the data;
    /// here we drop every selection/tool that belonged to the old
    /// room. No undo snapshot — callers snapshot when they should.
    fn activate_room(&mut self, id: u64) {
        if self.project.switch_room(id) {
            self.selected_item = None;
            self.selected_opening = None;
            self.selected_service = None;
            self.selected_note = None;
            self.placing = None;
            self.placing_note = false;
            self.drag_item = None;
            self.drag_guides = (Vec::new(), Vec::new());
            self.walking = false;
            self.calibrating = false;
            self.calib_marks.clear();
            self.status = format!("Now editing {}.", self.project.room.name);
        }
    }

    /// Switch rooms as its own undoable step.
    fn switch_to_room(&mut self, id: u64) {
        if id != self.project.active_room_id {
            self.snapshot();
            self.activate_room(id);
            self.mark_dirty();
        }
    }

    /// The room tab strip under the top bar (multi-room plans).
    fn room_tabs(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("room_tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Rooms:");
                let mut switch: Option<u64> = None;
                for (id, name) in self.project.room_list() {
                    let active = id == self.project.active_room_id;
                    if ui
                        .selectable_label(active, name)
                        .on_hover_text("Switch to this room")
                        .clicked()
                        && !active
                    {
                        switch = Some(id);
                    }
                }
                if ui
                    .button("+ Add room")
                    .on_hover_text("Add another room to this plan")
                    .clicked()
                {
                    self.gallery_for_room = true;
                    self.show_gallery = true;
                }
                if let Some(id) = switch {
                    self.switch_to_room(id);
                }
            });
        });
    }

    /// The template gallery: starter rooms as picture cards.
    fn gallery_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_gallery;
        let mut chosen: Option<(String, Project)> = None;
        let title = if self.gallery_for_room {
            "Add a room"
        } else {
            "Start from a template"
        };
        egui::Window::new(title)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                if self.gallery_for_room {
                    ui.label("Pick a starter for the new room — your other rooms stay exactly as they are.");
                } else {
                    ui.label("Pick a furnished starter room — everything in it can be moved, resized or deleted. (Undo brings your current plan back.)");
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let empty = Project::default();
                    if template_card(ui, "Empty room", "A blank 5 × 4 m room, ready to shape.", &empty) {
                        chosen = Some(("Empty room".into(), empty));
                    }
                    let living = starter_project();
                    if template_card(ui, "Living room", "Sofa, TV bench, coffee table, dining set.", &living) {
                        chosen = Some(("Living room".into(), living));
                    }
                });
                ui.horizontal(|ui| {
                    let bed = bedroom_project();
                    if template_card(ui, "Bedroom", "Bed, wardrobe, dresser and a lamp.", &bed) {
                        chosen = Some(("Bedroom".into(), bed));
                    }
                    let office = office_project();
                    if template_card(ui, "Home office", "Desk, chair, bookcase and storage.", &office) {
                        chosen = Some(("Home office".into(), office));
                    }
                });
            });
        if let Some((name, p)) = chosen {
            if self.gallery_for_room {
                self.snapshot();
                let id = self
                    .project
                    .add_room(roomforge::model::StoredRoom::from_project(&p));
                self.activate_room(id);
                self.status = format!("{name} added as a new room.");
                self.mark_dirty();
            } else {
                self.snapshot();
                self.project = p;
                self.selected_item = None;
                self.selected_opening = None;
                self.selected_service = None;
                self.current_path = None;
                self.status = format!("{name} template loaded.");
                self.mark_dirty();
            }
            self.gallery_for_room = false;
            open = false;
        }
        if !open {
            self.gallery_for_room = false;
        }
        self.show_gallery = open;
    }

    /// Save a PNG of the walkthrough, from where the visitor stands.
    fn save_walk_snapshot(&mut self) {
        let name = format!(
            "{}-walk",
            self.project
                .room
                .name
                .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-")
        );
        if let Some(path) = save_dialog("PNG image", "png", &name) {
            let img = roomforge::render::render_walk(
                &self.project,
                &self.walker,
                self.scene,
                self.style,
                1280,
                800,
            );
            match img.save(&path) {
                Ok(()) => self.status = format!("Walk view written to {}", path.display()),
                Err(e) => self.status = format!("Could not write {}: {e}", path.display()),
            }
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("RoomForge");
                ui.separator();
                ui.menu_button("New", |ui| {
                    if ui.button("Template gallery…").on_hover_text("See the starter rooms as pictures and pick one").clicked() {
                        self.show_gallery = true;
                        ui.close();
                    }
                    if ui.button("Add a room…").on_hover_text("Add another room to this plan and design it too").clicked() {
                        self.gallery_for_room = true;
                        self.show_gallery = true;
                        ui.close();
                    }
                    if ui.button("Empty room").on_hover_text("Start a new, empty room").clicked() {
                        self.snapshot();
                        self.project = Project::default();
                        self.selected_item = None;
                        self.current_path = None;
                        self.status = "New empty room.".into();
                        self.mark_dirty();
                        ui.close();
                    }
                    if ui.button("Template: Living room").on_hover_text("A furnished living room to rearrange").clicked() {
                        self.snapshot();
                        self.project = starter_project();
                        self.selected_item = None;
                        self.current_path = None;
                        self.status = "Living room template loaded.".into();
                        self.mark_dirty();
                        ui.close();
                    }
                    if ui.button("Template: Bedroom").on_hover_text("A furnished bedroom to rearrange").clicked() {
                        self.snapshot();
                        self.project = bedroom_project();
                        self.selected_item = None;
                        self.current_path = None;
                        self.status = "Bedroom template loaded.".into();
                        self.mark_dirty();
                        ui.close();
                    }
                    if ui.button("Template: Home office").on_hover_text("A furnished home office to rearrange").clicked() {
                        self.snapshot();
                        self.project = office_project();
                        self.selected_item = None;
                        self.current_path = None;
                        self.status = "Home office template loaded.".into();
                        self.mark_dirty();
                        ui.close();
                    }
                })
                .response
                .on_hover_text("Start a new room, empty or from a template");
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
                ui.separator();
                ui.label("Units");
                ui.selectable_value(&mut self.imperial, false, "Metric")
                    .on_hover_text("Show and type lengths in metres and centimetres");
                ui.selectable_value(&mut self.imperial, true, "Imperial")
                    .on_hover_text("Show and type lengths in feet and inches, e.g. 10' 6\"");
                ui.separator();
                if ui
                    .selectable_label(self.placing_note, "📝 Add note")
                    .on_hover_text("Click, then click in the room to pin a text note there")
                    .clicked()
                {
                    self.placing_note = true;
                    self.placing = None;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Export:");
                if ui.button("Plan (SVG)").on_hover_text("Save a to-scale drawing of the plan").clicked() {
                    if let Some(p) = save_dialog("SVG drawing", "svg", "plan") {
                        let svg = roomforge::export::plan_svg(&self.project, 110.0, self.imperial);
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
                        let html = roomforge::export::print_html(&self.project, self.imperial);
                        self.export_to(p, &html, "Print sheet");
                    }
                }
                ui.separator();
                if ui.button("Plan 1:50").on_hover_text("Save the plan as an SVG sized for true 1:50 printing — print it at 100%").clicked() {
                    if let Some(p) = save_dialog("SVG drawing", "svg", "plan-1-50") {
                        let svg = roomforge::export::plan_svg_scaled(&self.project, 50.0, self.imperial);
                        self.export_to(p, &svg, "Plan 1:50");
                    }
                }
                if ui.button("Plan 1:100").on_hover_text("Save the plan as an SVG sized for true 1:100 printing — print it at 100%").clicked() {
                    if let Some(p) = save_dialog("SVG drawing", "svg", "plan-1-100") {
                        let svg = roomforge::export::plan_svg_scaled(&self.project, 100.0, self.imperial);
                        self.export_to(p, &svg, "Plan 1:100");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Help")
                        .on_hover_text("Open the plain-language guide (F1)")
                        .clicked()
                    {
                        self.show_help = true;
                    }
                    if ui
                        .small_button("A+")
                        .on_hover_text("Make all text bigger")
                        .clicked()
                    {
                        self.text_scale = (self.text_scale + 0.1).min(1.5);
                    }
                    if ui
                        .small_button("A−")
                        .on_hover_text("Make all text smaller")
                        .clicked()
                    {
                        self.text_scale = (self.text_scale - 0.1).max(0.85);
                    }
                    ui.label("Text size");
                });
            });
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        let warnings = self.project.warnings();
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{} · {} × {} · {} · {} items",
                    self.project.room.name,
                    fmt_len(self.project.room.width_m, self.imperial),
                    fmt_len(self.project.room.depth_m, self.imperial),
                    fmt_area(self.project.room.area_m2(), self.imperial),
                    self.project.items.len()
                ));
                if self.project.room_count() > 1 {
                    ui.label(format!(
                        "· {} items in {} rooms",
                        self.project.total_items(),
                        self.project.room_count()
                    ));
                }
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
                ui.label("Click an item, then click in the room to place it — or drag it straight in. Drag placed items to move them.");
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
                                let selected = self.placing.as_ref().map(|p| p.catalog_id.as_str()) == Some(c.id);
                                let row = ui
                                    .horizontal(|ui| {
                                        let (gre, _) = ui.allocate_exact_size(
                                            egui::vec2(44.0, 44.0),
                                            egui::Sense::hover(),
                                        );
                                        draw_item_glyph(ui.painter(), gre, &c);
                                        ui.vertical(|ui| {
                                            ui.strong(c.name);
                                            ui.small(format!(
                                                "{} × {}",
                                                fmt_len(c.width_m, self.imperial),
                                                fmt_len(c.depth_m, self.imperial)
                                            ));
                                        });
                                    })
                                    .response
                                    .interact(egui::Sense::click_and_drag());
                                if selected {
                                    ui.painter().rect_stroke(
                                        row.rect,
                                        4.0,
                                        Stroke::new(1.5_f32, RED),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                let row = row.on_hover_text("Click, then click in the room to place it — or drag it straight in. Hold Shift while placing to place several.");
                                if row.clicked() {
                                    self.status = format!("Placing: {}. Click in the room.", c.name);
                                    self.placing = Some(PlaceTemplate::from(&c));
                                } else if row.drag_started() {
                                    self.status = format!("Placing: {}. Drop it in the room.", c.name);
                                    self.placing = Some(PlaceTemplate::from(&c));
                                    self.catalog_dragging = true;
                                }
                            }
                        });
                    }
                    self.custom_items_section(ui);
                    self.custom_models_section(ui);
                });
            });
    }

    /// Keep the parsed-model cache in step with the project's
    /// model definitions: parse new ones once, drop deleted ones.
    /// Parse failures are remembered so a broken def falls back to
    /// a box quietly instead of being retried every frame.
    fn sync_model_cache(&mut self) {
        let live: HashSet<u64> = self.project.custom_models.iter().map(|d| d.id).collect();
        self.model_cache.retain(|id, _| live.contains(id));
        self.model_failed.retain(|id| live.contains(id));
        for d in &self.project.custom_models {
            if !self.model_cache.contains_key(&d.id) && !self.model_failed.contains(&d.id) {
                match roomforge::model3d::prepare(d) {
                    Ok(p) => {
                        self.model_cache.insert(d.id, p);
                    }
                    Err(_) => {
                        self.model_failed.insert(d.id);
                    }
                }
            }
        }
    }

    /// Import a GLB/OBJ file as a placeable 3D model definition.
    fn import_model(&mut self) {
        let Some(path) = open_model_dialog() else {
            self.status = "Model import is available through the file dialog on Windows.".into();
            return;
        };
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "glb" && ext != "obj" {
            self.status = "Please choose a .glb or .obj model file.".into();
            return;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.status = format!("Could not read {}: {e}", path.display());
                return;
            }
        };
        if bytes.len() > 10 * 1024 * 1024 {
            self.status = "That model file is over 10 MB — plans carry their models inside the file, so please use a smaller export.".into();
            return;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Model")
            .to_string();
        use base64::Engine;
        let def_stub = roomforge::model::CustomModelDef {
            id: 0,
            name: stem.clone(),
            format: ext.clone(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
            unit_depth: 1.0,
            unit_height: 1.0,
            natural_width_m: 1.0,
        };
        let prep = match roomforge::model3d::prepare(&def_stub) {
            Ok(p) => p,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        // A unique display name: "Chair", "Chair (2)", …
        let mut name = stem;
        let mut n = 2;
        while self.project.custom_models.iter().any(|d| d.name == name) {
            name = format!("{} ({n})", def_stub.name);
            n += 1;
        }
        self.snapshot();
        let id = self.project.alloc_id();
        let w = prep.natural_width.clamp(0.2, 4.0);
        let first_rgb = prep
            .meshes
            .first()
            .map(|m| m.rgb)
            .unwrap_or([168, 162, 154]);
        self.project
            .custom_models
            .push(roomforge::model::CustomModelDef {
                id,
                name: name.clone(),
                format: ext,
                data_base64: def_stub.data_base64,
                unit_depth: prep.unit_depth,
                unit_height: prep.unit_height,
                natural_width_m: prep.natural_width,
            });
        self.model_cache.insert(id, prep.clone());
        self.placing = Some(PlaceTemplate {
            catalog_id: format!("model-{id}"),
            name: name.clone(),
            category: "3D models".into(),
            width_m: w,
            depth_m: w * prep.unit_depth,
            height_m: w * prep.unit_height,
            rgb: first_rgb,
        });
        self.status = format!("Model imported — placing: {name}. Click in the room.");
        self.mark_dirty();
    }

    /// "Your 3D models": imported GLB/OBJ definitions, placeable
    /// like catalog items. Definitions live in the project file.
    fn custom_models_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Your 3D models — imported")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Bring your own furniture: import a .glb or .obj model and it renders with real shape in the 3D view and walkthrough. In the 2D plan it shows as its footprint.");
                let mut place_def: Option<PlaceTemplate> = None;
                let mut delete_def: Option<u64> = None;
                for d in &self.project.custom_models {
                    ui.horizontal(|ui| {
                        let w = d.natural_width_m.clamp(0.2, 4.0);
                        let label = format!("{}  ({})", d.name, fmt_len(w, self.imperial));
                        let selected = self.placing.as_ref().map(|p| p.catalog_id.clone())
                            == Some(d.catalog_id());
                        if ui
                            .selectable_label(selected, label)
                            .on_hover_text("Click, then click in the room to place it")
                            .clicked()
                        {
                            place_def = Some(PlaceTemplate {
                                catalog_id: d.catalog_id(),
                                name: d.name.clone(),
                                category: "3D models".into(),
                                width_m: w,
                                depth_m: w * d.unit_depth,
                                height_m: w * d.unit_height,
                                rgb: self
                                    .model_cache
                                    .get(&d.id)
                                    .and_then(|p| p.meshes.first())
                                    .map(|m| m.rgb)
                                    .unwrap_or([168, 162, 154]),
                            });
                        }
                        if ui
                            .small_button("✕")
                            .on_hover_text(
                                "Remove this model (placed copies fall back to simple boxes)",
                            )
                            .clicked()
                        {
                            delete_def = Some(d.id);
                        }
                    });
                }
                if let Some(t) = place_def {
                    self.status = format!("Placing: {}. Click in the room.", t.name);
                    self.placing = Some(t);
                }
                if let Some(id) = delete_def {
                    self.snapshot();
                    self.project.custom_models.retain(|d| d.id != id);
                    self.mark_dirty();
                }
                if ui
                    .button("Import 3D model…")
                    .on_hover_text("Choose a .glb or .obj file (up to 10 MB) to add as furniture")
                    .clicked()
                {
                    self.import_model();
                }
            });
    }

    /// "Your items": the user's own furniture definitions, plus the
    /// builder form to create new ones. Definitions live in the
    /// project file, so they travel with the plan.
    fn custom_items_section(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        egui::CollapsingHeader::new("Your items — build your own")
            .default_open(false)
            .show(ui, |ui| {
                let mut place_def: Option<PlaceTemplate> = None;
                let mut delete_def: Option<u64> = None;
                for d in &self.project.custom_items {
                    ui.horizontal(|ui| {
                        let label = format!("{}  ({:.2}×{:.2} m)", d.name, d.width_m, d.depth_m);
                        let selected = self.placing.as_ref().map(|p| p.catalog_id.clone())
                            == Some(d.catalog_id());
                        if ui
                            .selectable_label(selected, label)
                            .on_hover_text("Click, then click in the room to place it")
                            .clicked()
                        {
                            place_def = Some(PlaceTemplate::from(d));
                        }
                        if ui
                            .small_button("✕")
                            .on_hover_text(
                                "Remove this definition (pieces already placed stay in the room)",
                            )
                            .clicked()
                        {
                            delete_def = Some(d.id);
                        }
                    });
                }
                if let Some(t) = place_def {
                    self.status = format!("Placing: {}. Click in the room.", t.name);
                    self.placing = Some(t);
                }
                if let Some(id) = delete_def {
                    self.snapshot();
                    self.project.custom_items.retain(|d| d.id != id);
                    self.mark_dirty();
                }
                ui.separator();
                ui.label("New custom item");
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.custom_name);
                });
                egui::ComboBox::from_label("Group")
                    .selected_text(self.custom_category.clone())
                    .show_ui(ui, |ui| {
                        for cat in categories() {
                            ui.selectable_value(&mut self.custom_category, cat.to_string(), cat);
                        }
                    });
                let imp = self.imperial;
                ui.horizontal(|ui| {
                    ui.label("Width");
                    len_edit(
                        ui,
                        "custom-w",
                        &mut self.custom_w,
                        imp,
                        "Width of your item",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Depth");
                    len_edit(
                        ui,
                        "custom-d",
                        &mut self.custom_d,
                        imp,
                        "Depth of your item",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Height");
                    len_edit(
                        ui,
                        "custom-h",
                        &mut self.custom_h,
                        imp,
                        "Height of your item",
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Colour");
                    ui.color_edit_button_srgb(&mut self.custom_rgb);
                });
                if ui
                    .button("Create & place")
                    .on_hover_text("Save this definition into the plan and start placing it")
                    .clicked()
                {
                    self.snapshot();
                    let id = self.project.alloc_id();
                    let mut def = roomforge::model::CustomItemDef {
                        id,
                        name: if self.custom_name.trim().is_empty() {
                            "Custom item".to_string()
                        } else {
                            self.custom_name.trim().to_string()
                        },
                        category: self.custom_category.clone(),
                        width_m: self.custom_w,
                        depth_m: self.custom_d,
                        height_m: self.custom_h,
                        rgb: self.custom_rgb,
                    };
                    def.clamp_dims();
                    self.placing = Some(PlaceTemplate::from(&def));
                    self.status = format!("Placing: {}. Click in the room.", def.name);
                    self.project.custom_items.push(def);
                    self.mark_dirty();
                }
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
                    ui.separator();
                    self.note_section(ui);
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
        // Whole-plan room management (multi-room plans).
        ui.horizontal(|ui| {
            if ui
                .button("Duplicate room")
                .on_hover_text("Copy this room and everything in it as a new room")
                .clicked()
            {
                self.snapshot();
                if let Some(nid) = self.project.duplicate_room(self.project.active_room_id) {
                    self.activate_room(nid);
                    self.status = "Room duplicated — you're editing the copy.".into();
                    self.mark_dirty();
                }
            }
            let last = self.project.room_count() <= 1;
            if ui
                .add_enabled(!last, egui::Button::new("Delete room"))
                .on_hover_text(if last {
                    "A plan always keeps at least one room"
                } else {
                    "Delete this room and everything in it (Undo brings it back)"
                })
                .clicked()
            {
                self.snapshot();
                let cur = self.project.active_room_id;
                if self.project.delete_room(cur) {
                    let now = self.project.active_room_id;
                    self.activate_room(now);
                    self.status = "Room deleted. Undo brings it back.".into();
                    self.mark_dirty();
                }
            }
        });
        let imp = self.imperial;
        macro_rules! dim {
            ($label:expr, $field:ident, $tip:expr) => {
                ui.horizontal(|ui| {
                    ui.label($label);
                    if len_edit(ui, $label, &mut self.project.room.$field, imp, $tip) {
                        changed = true;
                    }
                });
            };
        }
        dim!(
            "Width (W–E)",
            width_m,
            "Room width, west to east. Type a length, e.g. 5 m, 500 cm or 16' 5\""
        );
        dim!(
            "Depth (S–N)",
            depth_m,
            "Room depth, south to north. Type a length in the chosen units"
        );
        dim!(
            "Ceiling",
            height_m,
            "Ceiling height. Type a length in the chosen units"
        );
        dim!(
            "Wall thickness",
            wall_thickness_m,
            "Wall thickness, drawn on the plan (room sizes stay clear internal dimensions)"
        );
        ui.separator();
        ui.strong(format!("Room shape: {}", self.project.room.shape_label()));
        ui.label("Cut a rectangular notch out of a corner to make an L or U shape. Doors, windows and furniture adjust to fit.");
        ui.horizontal(|ui| {
            ui.label("Notch width");
            len_edit(
                ui,
                "notch-h",
                &mut self.notch_h,
                imp,
                "How far the notch runs along the top and bottom walls",
            );
        });
        ui.horizontal(|ui| {
            ui.label("Notch depth");
            len_edit(
                ui,
                "notch-v",
                &mut self.notch_v,
                imp,
                "How far the notch runs along the side walls",
            );
        });
        for corner in [
            Corner::NorthWest,
            Corner::NorthEast,
            Corner::SouthWest,
            Corner::SouthEast,
        ] {
            let cut = self.project.room.corner_is_cut(corner);
            ui.horizontal(|ui| {
                let btn = ui
                    .add_enabled(
                        !cut,
                        egui::Button::new(format!("Cut {} corner", corner.label())),
                    )
                    .on_hover_text(if cut {
                        "This corner is already cut — restore the rectangle to redo it"
                    } else {
                        "Cut the notch described above out of this corner"
                    });
                if btn.clicked() {
                    let mut trial = self.project.room.clone();
                    match trial.cut_notch(corner, self.notch_h, self.notch_v) {
                        Ok(()) => {
                            self.snapshot();
                            self.project.room = trial;
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
                            for a in &mut self.project.annotations {
                                a.clamp_inside(&room);
                            }
                            self.status = format!(
                                "{} corner cut — the room is now {}.",
                                corner.label(),
                                self.project.room.shape_label()
                            );
                            self.mark_dirty();
                        }
                        Err(e) => {
                            self.status = e;
                        }
                    }
                }
                if cut {
                    ui.label("(cut)");
                }
            });
        }
        if !self.project.room.outline.is_empty()
            && ui
                .button("Restore rectangle")
                .on_hover_text("Remove all notches and go back to a plain rectangle")
                .clicked()
        {
            self.snapshot();
            self.project.room.clear_outline();
            let room = self.project.room.clone();
            for o in &mut self.project.openings {
                o.clamp_to_wall(&room);
            }
            for s in &mut self.project.services {
                s.clamp_to_wall(&room);
            }
            self.status = "Back to a plain rectangle.".into();
            self.mark_dirty();
        }
        ui.horizontal(|ui| {
            ui.label("Wall colour");
            if ui
                .color_edit_button_srgb(&mut self.project.room.wall_rgb)
                .changed()
            {
                changed = true;
            }
        });
        ui.label("Paint swatches — click to apply to the walls:");
        ui.horizontal_wrapped(|ui| {
            for (name, col) in [
                ("Warm white", [216, 210, 200]),
                ("Pure white", [240, 240, 238]),
                ("Greige", [178, 168, 156]),
                ("Sage", [156, 168, 148]),
                ("Slate blue", [110, 128, 148]),
                ("Terracotta", [188, 120, 92]),
                ("Charcoal", [70, 70, 74]),
                ("Family red", [107, 7, 0]),
            ] {
                if ui
                    .add(egui::Button::new("  ").fill(rgb(col)))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.project.room.wall_rgb = col;
                    changed = true;
                }
            }
        });
        ui.separator();
        ui.strong("Flooring");
        let mut material = self.project.room.floor_material;
        egui::ComboBox::from_label("Material")
            .selected_text(material.label())
            .show_ui(ui, |ui| {
                for m in roomforge::model::FloorMaterial::all() {
                    ui.selectable_value(&mut material, m, m.label());
                }
            });
        if material != self.project.room.floor_material {
            self.project.room.floor_material = material;
            if let Some(base) = material.base_rgb() {
                self.project.room.floor_rgb = base;
            }
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label("Floor colour");
            if ui
                .color_edit_button_srgb(&mut self.project.room.floor_rgb)
                .changed()
            {
                changed = true;
            }
        });
        ui.label("Floor swatches (a custom colour makes the floor solid):");
        ui.horizontal_wrapped(|ui| {
            for (name, col) in [
                ("Natural wood", [148, 112, 76]),
                ("Dark wood", [90, 64, 44]),
                ("Pale tile", [200, 200, 198]),
                ("Mid grey", [120, 120, 120]),
                ("Dark slate", [60, 60, 64]),
            ] {
                if ui
                    .add(egui::Button::new("  ").fill(rgb(col)))
                    .on_hover_text(name)
                    .clicked()
                {
                    self.project.room.floor_rgb = col;
                    self.project.room.floor_material = roomforge::model::FloorMaterial::Solid;
                    changed = true;
                }
            }
        });
        ui.separator();
        egui::CollapsingHeader::new("Blueprint tracing — lay a floor-plan image under your drawing")
            .default_open(false)
            .show(ui, |ui| {
            if self.project.underlay.is_none() {
                if ui
                    .button("Load plan image…")
                    .on_hover_text("Lay a scanned or photographed floor plan under the drawing and trace over it. The image is stored inside the plan file")
                    .clicked()
                {
                    if let Some(p) = open_image_dialog() {
                        self.load_underlay(p);
                    } else {
                        self.status = "Loading an image uses the file dialog on Windows.".into();
                    }
                }
            } else {
                if let Some(u) = &mut self.project.underlay {
                    ui.add(egui::Slider::new(&mut u.opacity, 0.15..=1.0).text("Image opacity"))
                        .on_hover_text("How strongly the tracing image shows through");
                }
                if ui
                    .selectable_label(self.calibrating, "Calibrate scale")
                    .on_hover_text("Click two points on the image whose real distance you know (e.g. a doorway), type that distance, and the image rescales to match")
                    .clicked()
                {
                    self.calibrating = true;
                    self.calib_marks.clear();
                    self.placing = None;
                    self.placing_note = false;
                }
                if self.calibrating {
                    ui.label(format!(
                        "Click point {} of 2 on the plan image.",
                        self.calib_marks.len() + 1
                    ));
                    ui.horizontal(|ui| {
                        ui.label("Real distance");
                        len_edit(
                            ui,
                            "calib-dist",
                            &mut self.calib_dist_m,
                            imp,
                            "The true distance between the two points you click",
                        );
                    });
                    if ui.button("Cancel calibration").clicked() {
                        self.calibrating = false;
                        self.calib_marks.clear();
                    }
                }
                if ui
                    .button("Remove image")
                    .on_hover_text("Take the tracing image out of this plan")
                    .clicked()
                {
                    self.snapshot();
                    self.project.underlay = None;
                    self.underlay_tex = None;
                    self.calibrating = false;
                    self.mark_dirty();
                }
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
            for a in &mut self.project.annotations {
                a.clamp_inside(&room);
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
                    links_to: None,
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
                    links_to: None,
                });
                self.selected_opening = Some(id);
                self.mark_dirty();
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (kind, label) in [
                (ServiceKind::Socket, "＋ Socket"),
                (ServiceKind::Pipe, "＋ Pipe"),
                (ServiceKind::Radiator, "＋ Radiator"),
                (ServiceKind::Switch, "＋ Switch"),
                (ServiceKind::TvPoint, "＋ TV point"),
                (ServiceKind::Network, "＋ Network"),
                (ServiceKind::Thermostat, "＋ Thermostat"),
            ] {
                if ui
                    .button(label)
                    .on_hover_text("Add this service point to the chosen wall")
                    .clicked()
                {
                    self.snapshot();
                    let id = self.project.alloc_id();
                    let (rs, re) = self.wall_choice.longest_run(&self.project.room);
                    self.project.services.push(ServicePoint {
                        id,
                        kind,
                        wall: self.wall_choice,
                        offset_m: (rs + re) / 2.0,
                    });
                    self.selected_service = Some(id);
                    self.mark_dirty();
                }
            }
        });
        // Editors for the currently selected opening / service.
        // Door link targets (other rooms), computed before the
        // mutable borrow of the opening itself.
        let link_targets: Vec<(u64, String)> = self
            .project
            .room_list()
            .into_iter()
            .filter(|(id, _)| *id != self.project.active_room_id)
            .collect();
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
                    if !link_targets.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Leads to");
                            let cur = o
                                .links_to
                                .and_then(|t| {
                                    link_targets
                                        .iter()
                                        .find(|(id, _)| *id == t)
                                        .map(|(_, n)| n.clone())
                                })
                                .unwrap_or_else(|| "—".into());
                            egui::ComboBox::from_id_salt(format!("leads-{oid}"))
                                .selected_text(cur)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(o.links_to.is_none(), "— (not set)")
                                        .clicked()
                                    {
                                        o.links_to = None;
                                    }
                                    for (tid, tname) in &link_targets {
                                        if ui
                                            .selectable_label(o.links_to == Some(*tid), tname)
                                            .clicked()
                                        {
                                            o.links_to = Some(*tid);
                                        }
                                    }
                                });
                        });
                    }
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

    /// The Saved views window: recall a stored presentation
    /// (camera + lighting + style + wall cut), delete one, or save
    /// the current setup under a name. Views live in the plan.
    fn views_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut apply: Option<SavedView> = None;
        let mut delete: Option<u64> = None;
        egui::Window::new("Saved views")
            .open(&mut open)
            .resizable(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.label(
                    "A saved view remembers the camera, lighting, style and wall cut. \
                     Click one to jump back to it — views travel with the plan.",
                );
                ui.add_space(4.0);
                if self.project.views.is_empty() {
                    ui.label("No saved views yet. Set up a view you like in the 3D tab, then save it below.");
                }
                for v in &self.project.views {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(false, &v.name)
                            .on_hover_text("Show this view")
                            .clicked()
                        {
                            apply = Some(v.clone());
                        }
                        if ui
                            .small_button("✕")
                            .on_hover_text("Delete this view")
                            .clicked()
                        {
                            delete = Some(v.id);
                        }
                    });
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.view_name);
                });
                if ui
                    .button("Save the current view")
                    .on_hover_text("Store the camera, lighting, style and wall cut exactly as they are now")
                    .clicked()
                {
                    let name = if self.view_name.trim().is_empty() {
                        format!("View {}", self.project.views.len() + 1)
                    } else {
                        self.view_name.trim().to_string()
                    };
                    self.snapshot();
                    let id = self.project.alloc_id();
                    self.project.views.push(SavedView {
                        id,
                        name,
                        yaw_deg: self.camera.yaw_deg,
                        pitch_deg: self.camera.pitch_deg,
                        scale: self.camera.scale,
                        scene: self.scene,
                        style: self.style,
                        wall_cut: self.wall_cut,
                    });
                    self.view_name.clear();
                    self.mark_dirty();
                    self.status = "View saved — it travels with the plan.".into();
                }
            });
        if !open {
            self.show_views = false;
        }
        if let Some(v) = apply {
            self.camera.yaw_deg = v.yaw_deg;
            self.camera.pitch_deg = v.pitch_deg;
            self.camera.scale = v.scale;
            self.scene = v.scene;
            self.style = v.style;
            self.wall_cut = v.wall_cut;
            self.view = View::ThreeD;
            self.walking = false;
            self.show_views = false;
            self.status = format!("View “{}” applied.", v.name);
        }
        if let Some(id) = delete {
            self.snapshot();
            self.project.views.retain(|v| v.id != id);
            self.mark_dirty();
        }
    }

    fn selection_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Selected furniture");
        let Some(id) = self.selected_item else {
            ui.label("Nothing selected. Click an item in the plan.");
            return;
        };
        let mut delete = false;
        let mut duplicate = false;
        let model_name = self
            .project
            .items
            .iter()
            .find(|i| i.id == id)
            .and_then(|it| self.project.model_def(&it.catalog_id))
            .map(|d| d.name.clone());
        if let Some(it) = self.project.items.iter_mut().find(|i| i.id == id) {
            if let Some(name) = &model_name {
                ui.label(format!(
                    "3D model: {name} — real shape in the 3D views; the plan shows its footprint."
                ));
            }
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut it.name);
            });
            let imp = self.imperial;
            macro_rules! fdim {
                ($label:expr, $field:ident) => {
                    ui.horizontal(|ui| {
                        ui.label($label);
                        len_edit(
                            ui,
                            $label,
                            &mut it.$field,
                            imp,
                            "Type a length in the chosen units",
                        );
                    });
                };
            }
            fdim!("Centre — from left wall", x_m);
            fdim!("Centre — from front wall", y_m);
            fdim!("Width", width_m);
            fdim!("Depth", depth_m);
            fdim!("Height", height_m);
            let (cw, ce, cs, cn) = it.wall_clearances(&self.project.room);
            ui.separator();
            ui.strong("Space around it")
                .on_hover_text("Empty space between this item's edges and each wall — handy for checking walkways and door clearance");
            egui::Grid::new("clearances").num_columns(2).show(ui, |ui| {
                ui.label("To the left wall");
                ui.label(fmt_len(cw, imp));
                ui.end_row();
                ui.label("To the right wall");
                ui.label(fmt_len(ce, imp));
                ui.end_row();
                ui.label("To the front wall");
                ui.label(fmt_len(cs, imp));
                ui.end_row();
                ui.label("To the back wall");
                ui.label(fmt_len(cn, imp));
                ui.end_row();
            });
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
            ui.label("Keys: arrow keys nudge it · R turns it · Delete removes it. (Full guide: Help, or F1.)");
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

    fn note_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Plan note");
        let Some(id) = self.selected_note else {
            ui.label("No note selected. Use 📝 Add note, then click in the room.");
            return;
        };
        let mut delete = false;
        let mut edited = false;
        if let Some(a) = self.project.annotations.iter_mut().find(|a| a.id == id) {
            ui.label("Text");
            if ui.text_edit_multiline(&mut a.text).changed() {
                edited = true;
            }
            let imp = self.imperial;
            ui.horizontal(|ui| {
                ui.label("X");
                len_edit(ui, "note-x", &mut a.x_m, imp, "Distance from the west wall");
            });
            ui.horizontal(|ui| {
                ui.label("Y");
                len_edit(
                    ui,
                    "note-y",
                    &mut a.y_m,
                    imp,
                    "Distance from the south wall",
                );
            });
            if ui
                .button("Delete note")
                .on_hover_text("Remove this note (Delete key)")
                .clicked()
            {
                delete = true;
            }
        } else {
            self.selected_note = None;
        }
        if edited {
            self.mark_dirty();
        }
        if delete {
            self.snapshot();
            self.project.annotations.retain(|a| a.id != id);
            self.selected_note = None;
            self.mark_dirty();
        }
    }

    /// The note (if any) within 0.35 m of a room point.
    fn note_at(&self, rx: f64, ry: f64) -> Option<u64> {
        self.project
            .annotations
            .iter()
            .rev()
            .find(|a| (a.x_m - rx).abs() < 0.35 && (a.y_m - ry).abs() < 0.35)
            .map(|a| a.id)
    }

    // ---------- 2D plan ----------

    fn plan_view(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.interact(rect, ui.id().with("plan"), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(0x08, 0x08, 0x08));

        let room = self.project.room.clone();
        let margin = 58.0f32;
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
        // Floor + walls. The floor is filled cell-by-cell so concave
        // (notched) outlines render correctly; walls are the outline
        // edges stroked at the wall's real thickness.
        let outline = room.outline_or_rect();
        let floor_cells = roomforge::geometry::rect_cells(&outline);
        for (x1, y1, x2, y2) in &floor_cells {
            painter.rect_filled(
                Rect::from_two_pos(to_screen(*x1, *y2), to_screen(*x2, *y1)),
                0.0,
                rgb(room.floor_rgb),
            );
        }
        // Flooring pattern (planks / tiles / joints), clipped to the
        // floor by construction — segments are generated per cell.
        let pattern_col = rgb(shade_scene(room.floor_rgb, 0.7, Scene::Day));
        for (a, b) in room.floor_material.pattern_segments(&floor_cells) {
            painter.line_segment(
                [to_screen(a.0, a.1), to_screen(b.0, b.1)],
                Stroke::new(1.0_f32, pattern_col),
            );
        }
        // Tracing underlay: decode once per image, draw over the floor.
        let need_tex = match (&self.project.underlay, &self.underlay_tex) {
            (Some(u), Some((k, _))) => *k != (u.data_b64.len(), u.width_px, u.height_px),
            (Some(_), None) => true,
            _ => false,
        };
        if need_tex {
            use base64::Engine;
            let b64 = self.project.underlay.as_ref().map(|u| u.data_b64.clone());
            let decoded = b64
                .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .map(|img| {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
                });
            if let (Some(ci), Some(u)) = (decoded, &self.project.underlay) {
                let key = (u.data_b64.len(), u.width_px, u.height_px);
                let tex = ui
                    .ctx()
                    .load_texture("underlay", ci, egui::TextureOptions::LINEAR);
                self.underlay_tex = Some((key, tex));
            }
        }
        if let (Some(u), Some((_, tex))) = (&self.project.underlay, &self.underlay_tex) {
            let tl = to_screen(u.origin_x_m, u.origin_y_m);
            let br_room = u.room_from_px(u.width_px as f64, u.height_px as f64);
            let br = to_screen(br_room.0, br_room.1);
            painter.image(
                tex.id(),
                Rect::from_two_pos(tl, br),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    (u.opacity.clamp(0.0, 1.0) * 255.0) as u8,
                ),
            );
        }
        let wall_stroke = Stroke::new(
            (room.wall_thickness_m as f32 * scale).max(2.5),
            Color32::from_rgb(230, 230, 230),
        );
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            painter.line_segment([to_screen(a.0, a.1), to_screen(b.0, b.1)], wall_stroke);
        }
        // Dimension lines: width below the south wall, depth left of the west.
        let font = FontId::proportional(13.0);
        let dim_col = Color32::LIGHT_GRAY;
        let dim_y = origin.y + 16.0;
        painter.line_segment(
            [
                Pos2::new(origin.x, dim_y),
                Pos2::new(origin.x + room.width_m as f32 * scale, dim_y),
            ],
            Stroke::new(1.0_f32, dim_col),
        );
        for tx in [origin.x, origin.x + room.width_m as f32 * scale] {
            painter.line_segment(
                [Pos2::new(tx, dim_y - 5.0), Pos2::new(tx, dim_y + 5.0)],
                Stroke::new(1.0_f32, dim_col),
            );
        }
        painter.text(
            Pos2::new(origin.x + room.width_m as f32 * scale / 2.0, dim_y + 6.0),
            Align2::CENTER_TOP,
            fmt_len(room.width_m, self.imperial),
            font.clone(),
            dim_col,
        );
        let dim_x = origin.x - 16.0;
        painter.line_segment(
            [
                Pos2::new(dim_x, origin.y),
                Pos2::new(dim_x, origin.y - room.depth_m as f32 * scale),
            ],
            Stroke::new(1.0_f32, dim_col),
        );
        for ty in [origin.y, origin.y - room.depth_m as f32 * scale] {
            painter.line_segment(
                [Pos2::new(dim_x - 5.0, ty), Pos2::new(dim_x + 5.0, ty)],
                Stroke::new(1.0_f32, dim_col),
            );
        }
        painter.text(
            Pos2::new(dim_x - 8.0, origin.y - room.depth_m as f32 * scale / 2.0),
            Align2::RIGHT_CENTER,
            fmt_len(room.depth_m, self.imperial),
            font,
            dim_col,
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

        // Annotations: numbered red markers with their text.
        for (i, a) in self.project.annotations.iter().enumerate() {
            let c = to_screen(a.x_m, a.y_m);
            let selected = self.selected_note == Some(a.id);
            painter.circle_filled(c, 9.0, RED);
            if selected {
                painter.circle_stroke(c, 11.0, Stroke::new(2.0_f32, Color32::WHITE));
            }
            painter.text(
                c,
                Align2::CENTER_CENTER,
                format!("{}", i + 1),
                FontId::proportional(11.0),
                Color32::WHITE,
            );
            let short: String = a.text.chars().take(28).collect();
            painter.text(
                Pos2::new(c.x + 13.0, c.y),
                Align2::LEFT_CENTER,
                short,
                FontId::proportional(11.0),
                Color32::LIGHT_GRAY,
            );
        }

        // Alignment guides + live gap readout while dragging.
        if self.drag_item.is_some() {
            let guide_col = Color32::from_rgba_unmultiplied(229, 72, 77, 170);
            for gx in &self.drag_guides.0 {
                painter.line_segment(
                    [to_screen(*gx, 0.0), to_screen(*gx, room.depth_m)],
                    Stroke::new(1.5_f32, guide_col),
                );
            }
            for gy in &self.drag_guides.1 {
                painter.line_segment(
                    [to_screen(0.0, *gy), to_screen(room.width_m, *gy)],
                    Stroke::new(1.5_f32, guide_col),
                );
            }
            if let Some(it) = self
                .project
                .items
                .iter()
                .find(|i| Some(i.id) == self.drag_item)
            {
                let (w, e, s, n) = it.wall_clearances(&room);
                let imp = self.imperial;
                painter.text(
                    rect.min + egui::vec2(10.0, 8.0),
                    egui::Align2::LEFT_TOP,
                    format!(
                        "Gaps — left {} · right {} · front {} · back {}",
                        fmt_len(w, imp),
                        fmt_len(e, imp),
                        fmt_len(s, imp),
                        fmt_len(n, imp)
                    ),
                    FontId::proportional(13.0),
                    Color32::from_rgb(245, 245, 245),
                );
            }
        }

        // Placement ghost / interactions.
        if let Some(pos) = response.interact_pointer_pos() {
            let (rx, ry) = to_room(pos);
            if self.calibrating {
                if let Some(u) = &self.project.underlay {
                    for m in &self.calib_marks {
                        let rp = u.room_from_px(m.0, m.1);
                        painter.circle_filled(to_screen(rp.0, rp.1), 5.0, RED);
                        painter.circle_stroke(
                            to_screen(rp.0, rp.1),
                            8.0,
                            Stroke::new(1.5_f32, Color32::WHITE),
                        );
                    }
                }
                if response.clicked() {
                    let px = self
                        .project
                        .underlay
                        .as_ref()
                        .map(|u| u.px_from_room(rx, ry));
                    if let Some(px) = px {
                        self.calib_marks.push(px);
                        if self.calib_marks.len() == 2 {
                            self.snapshot();
                            let (a, b) = (self.calib_marks[0], self.calib_marks[1]);
                            let d = self.calib_dist_m;
                            if let Some(u2) = &mut self.project.underlay {
                                u2.calibrate(a, b, d);
                            }
                            self.calib_marks.clear();
                            self.calibrating = false;
                            self.status = "Plan image scale calibrated.".into();
                            self.mark_dirty();
                        }
                    }
                }
                return;
            }
            if self.placing_note {
                painter.circle_stroke(pos, 9.0, Stroke::new(2.0_f32, RED));
                if response.clicked() {
                    self.snapshot();
                    let id = self.project.alloc_id();
                    let mut note = Annotation {
                        id,
                        x_m: rx,
                        y_m: ry,
                        text: "Note".into(),
                    };
                    let room2 = self.project.room.clone();
                    note.clamp_inside(&room2);
                    self.project.annotations.push(note);
                    self.selected_note = Some(id);
                    self.selected_item = None;
                    self.mark_dirty();
                    if !ui.input(|i| i.modifiers.shift) {
                        self.placing_note = false;
                    }
                }
                return;
            }
            if let Some(cat) = self.placing.clone() {
                // Ghost preview.
                let fake = PlacedItem {
                    id: 0,
                    catalog_id: cat.catalog_id.clone(),
                    name: cat.name.clone(),
                    category: cat.category.clone(),
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
                let dropped = self.catalog_dragging && ui.ctx().input(|i| i.pointer.any_released());
                if dropped {
                    self.catalog_dragging = false;
                }
                if response.clicked() || dropped {
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
                    let (mut nx, mut ny) = (rx + ox, ry + oy);
                    // Magnetic alignment to other items and the room
                    // first; the 5 cm grid fills in per axis where no
                    // alignment caught.
                    let mut guides = (Vec::new(), Vec::new());
                    if let Some(cur) = self.project.items.iter().find(|i| i.id == id) {
                        let bb = cur.aabb();
                        let shifted = (
                            bb.0 + (nx - cur.x_m),
                            bb.1 + (ny - cur.y_m),
                            bb.2 + (nx - cur.x_m),
                            bb.3 + (ny - cur.y_m),
                        );
                        let others: Vec<(f64, f64, f64, f64)> = self
                            .project
                            .items
                            .iter()
                            .filter(|i| i.id != id)
                            .map(|i| i.aabb())
                            .collect();
                        let al = align_snap(
                            shifted,
                            &others,
                            self.project.room.width_m,
                            self.project.room.depth_m,
                            0.09,
                        );
                        nx += al.dx;
                        ny += al.dy;
                        guides = (al.guides_x, al.guides_y);
                        if self.snap_enabled {
                            if al.dx == 0.0 {
                                nx = snap(nx, 0.05);
                            }
                            if al.dy == 0.0 {
                                ny = snap(ny, 0.05);
                            }
                        }
                    } else if self.snap_enabled {
                        nx = snap(nx, 0.05);
                        ny = snap(ny, 0.05);
                    }
                    self.drag_guides = guides;
                    if let Some(it) = self.project.items.iter_mut().find(|i| i.id == id) {
                        it.x_m = nx;
                        it.y_m = ny;
                        let room2 = self.project.room.clone();
                        it.clamp_inside(&room2);
                    }
                    self.mark_dirty();
                }
            }
            if response.drag_stopped() {
                self.drag_item = None;
                self.drag_snapshot_done = false;
                self.drag_guides = (Vec::new(), Vec::new());
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
                if let Some(nid) = self.note_at(rx, ry) {
                    self.selected_note = Some(nid);
                    self.selected_item = None;
                } else if hit.is_none() {
                    self.selected_item = None;
                    self.selected_note = None;
                    self.select_fitting_at(rx, ry);
                } else {
                    self.selected_item = hit;
                    self.selected_note = None;
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
            let (label, fill_rgb) = s.kind.symbol();
            let pos = to_screen(cx, cy);
            let selected = self.selected_service == Some(s.id);
            painter.circle_filled(pos, 10.0, rgb(fill_rgb));
            if selected {
                painter.circle_stroke(pos, 13.0, Stroke::new(2.0_f32, RED));
            }
            painter.text(
                pos,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(if label.len() > 1 { 8.0 } else { 11.0 }),
                Color32::from_rgb(10, 10, 10),
            );
        }
    }

    // ---------- 3D view ----------

    /// First-person walk mode (inside the 3D view): WASD/arrows
    /// move with collision, drag or Q/E look around, Esc leaves.
    fn walk_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("← Back to the orbit view")
                .on_hover_text("Leave the walkthrough (Esc works too)")
                .clicked()
            {
                self.walking = false;
            }
            ui.separator();
            ui.label("Lighting");
            for (s, tip) in [
                (Scene::Day, "Bright daylight"),
                (Scene::Evening, "Warm, softer evening light"),
                (
                    Scene::Night,
                    "Dim night, with cool light through the windows",
                ),
            ] {
                if ui
                    .selectable_label(self.scene == s, s.label())
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.scene = s;
                }
            }
            ui.separator();
            ui.label("Style");
            for (st, tip) in [
                (Style::Coloured, "The room's real colours"),
                (
                    Style::Clay,
                    "Everything in warm grey — good for judging shapes",
                ),
            ] {
                if ui
                    .selectable_label(
                        self.style == st,
                        match st {
                            Style::Coloured => "Coloured",
                            Style::Clay => "Clay",
                        },
                    )
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.style = st;
                }
            }
            ui.separator();
            if ui
                .button("Save PNG of this view")
                .on_hover_text("Render the view from where you're standing to a PNG picture")
                .clicked()
            {
                self.save_walk_snapshot();
            }
        });
        ui.label("W A S D or the arrow keys move you · Q / E or dragging turns your head · Shift walks faster · Esc steps back out.");

        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        let ctx = ui.ctx().clone();
        if response.dragged() {
            let d = response.drag_delta();
            self.walker.yaw_deg = (self.walker.yaw_deg + d.x as f64 * 0.25).rem_euclid(360.0);
            self.walker.pitch_deg = (self.walker.pitch_deg - d.y as f64 * 0.25).clamp(-60.0, 60.0);
        }
        let dt = ctx.input(|i| (i.stable_dt as f64).min(0.05));
        let (fwd_in, back_in, left_in, right_in, turn_l, turn_r, fast, esc) = ctx.input(|i| {
            (
                i.key_down(egui::Key::W) || i.key_down(egui::Key::ArrowUp),
                i.key_down(egui::Key::S) || i.key_down(egui::Key::ArrowDown),
                i.key_down(egui::Key::A) || i.key_down(egui::Key::ArrowLeft),
                i.key_down(egui::Key::D) || i.key_down(egui::Key::ArrowRight),
                i.key_down(egui::Key::Q),
                i.key_down(egui::Key::E),
                i.modifiers.shift,
                i.key_pressed(egui::Key::Escape),
            )
        });
        if esc {
            self.walking = false;
            return;
        }
        if turn_l {
            self.walker.yaw_deg = (self.walker.yaw_deg - 100.0 * dt).rem_euclid(360.0);
        }
        if turn_r {
            self.walker.yaw_deg = (self.walker.yaw_deg + 100.0 * dt).rem_euclid(360.0);
        }
        let fwd = (fwd_in as i32 - back_in as i32) as f64;
        let strafe = (right_in as i32 - left_in as i32) as f64;
        if fwd != 0.0 || strafe != 0.0 {
            let speed = if fast { 3.0 } else { 1.6 };
            let f = self.walker.forward();
            let r = self.walker.right();
            let dx = (f.0 * fwd + r.0 * strafe) * speed * dt;
            let dy = (f.1 * fwd + r.1 * strafe) * speed * dt;
            self.walker.pos = self.walker.moved(&self.project, dx, dy);
        }
        ctx.request_repaint();

        let painter = ui.painter_at(rect);
        let bg = self.scene.background();
        painter.rect_filled(rect, 0.0, Color32::from_rgb(bg[0], bg[1], bg[2]));
        let layers = walk_layers(
            &self.project,
            &self.walker,
            self.scene,
            self.style,
            rect.width() as f64,
            rect.height() as f64,
            &self.model_cache,
        );
        for layer in layers {
            match layer.cmd {
                WalkCmd::Fill { pts, color } => {
                    let pts: Vec<Pos2> = pts
                        .iter()
                        .map(|&(x, y)| rect.min + egui::vec2(x as f32, y as f32))
                        .collect();
                    if pts.len() >= 3 {
                        painter.add(Shape::convex_polygon(
                            pts,
                            rgb(color),
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 60)),
                        ));
                    }
                }
                WalkCmd::Line { a, b, color } => {
                    painter.line_segment(
                        [
                            rect.min + egui::vec2(a.0 as f32, a.1 as f32),
                            rect.min + egui::vec2(b.0 as f32, b.1 as f32),
                        ],
                        Stroke::new(1.0_f32, rgb(color)),
                    );
                }
            }
        }
    }

    fn three_d_view(&mut self, ui: &mut egui::Ui) {
        if self.walking {
            self.walk_view(ui);
            return;
        }
        ui.horizontal(|ui| {
            ui.label("Orbit");
            ui.add(egui::Slider::new(&mut self.camera.yaw_deg, 0.0..=360.0).text("Turn"))
                .on_hover_text("Turn the view around the room");
            ui.add(egui::Slider::new(&mut self.camera.pitch_deg, 15.0..=80.0).text("Tilt"))
                .on_hover_text("Tilt the view — low for eye level, high for looking down");
            ui.add(egui::Slider::new(&mut self.camera.scale, 25.0..=160.0).text("Zoom"));
        });
        ui.horizontal(|ui| {
            ui.label("Drag the view to orbit.");
            ui.separator();
            ui.label("Lighting");
            for (s, tip) in [
                (Scene::Day, "Bright daylight"),
                (Scene::Evening, "Warm, softer evening light"),
                (Scene::Night, "Dim night, with cool light through the windows"),
            ] {
                if ui
                    .selectable_label(self.scene == s, s.label())
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.scene = s;
                }
            }
            if ui
                .button("Save PNG snapshot")
                .on_hover_text("Render the standard 3D view to a PNG picture you can share (rendered offline, no screenshot needed)")
                .clicked()
            {
                self.save_snapshot();
            }
            ui.separator();
            if ui
                .button("Walk through the room")
                .on_hover_text("Stand inside the room and walk around it — W A S D to move, drag to look")
                .clicked()
            {
                self.walker = Walker::start(&self.project);
                self.walking = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Style");
            for (st, label, tip) in [
                (Style::Coloured, "Coloured", "The room's real colours"),
                (Style::Clay, "Clay", "Everything in warm grey — good for judging shapes and layout"),
            ] {
                if ui
                    .selectable_label(self.style == st, label)
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.style = st;
                }
            }
            ui.separator();
            ui.label("Walls");
            for (c, label, tip) in [
                (WallCut::Full, "Full", "Walls at their full height"),
                (WallCut::Half, "Half", "Walls cut to just over half height, so you can see in"),
                (WallCut::Low, "Low", "Walls cut low — a dollhouse view of the layout"),
            ] {
                if ui
                    .selectable_label(self.wall_cut == c, label)
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.wall_cut = c;
                }
            }
            ui.separator();
            if ui
                .button("Saved views…")
                .on_hover_text("Save this camera, lighting, style and wall cut under a name, and jump back to it any time")
                .clicked()
            {
                self.show_views = true;
            }
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

        // Gather everything to paint, depth-sorted far → near: face
        // fills plus floor-pattern lines (biased a hair nearer than
        // the floor itself, so lines sit on the floor and stay under
        // furniture and walls — the same layering as the PNG render).
        let mut draws: Vec<(f64, Draw)> = Vec::new();
        let outline = room.outline_or_rect();
        let floor_cells = roomforge::geometry::rect_cells(&outline);
        // Floor: rectangle cells of the outline (handles notches).
        for (x1, y1, x2, y2) in &floor_cells {
            let corners = [
                (*x1, *y1, 0.0),
                (*x2, *y1, 0.0),
                (*x2, *y2, 0.0),
                (*x1, *y2, 0.0),
            ];
            let (pts, depth) = face_of(&corners, &pr);
            draws.push((
                depth,
                Draw::Fill(
                    pts,
                    rgb(shade_scene(
                        style_rgb(room.floor_rgb, self.style),
                        0.9,
                        self.scene,
                    )),
                ),
            ));
        }
        let pattern_col = rgb(shade_scene(
            style_rgb(room.floor_rgb, self.style),
            0.7,
            self.scene,
        ));
        for (a, b) in room.floor_material.pattern_segments(&floor_cells) {
            let (pa, da) = pr((a.0, a.1, 0.0));
            let (pb, db) = pr((b.0, b.1, 0.0));
            draws.push(((da + db) / 2.0 - 0.02, Draw::Line([pa, pb], pattern_col)));
        }
        // Walls: one quad per outline edge, shaded by the direction
        // the wall faces (the outline is CCW, so the outward normal
        // is to the right of the direction of travel). The wall cut
        // (Full/Half/Low) caps the drawn height.
        let wall_top = self.wall_cut.cut_height(room.height_m);
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let (nx, ny) = (dy, -dx); // outward normal (CCW outline)
            let bright = if ny.abs() > nx.abs() {
                if ny > 0.0 {
                    0.95
                } else {
                    0.75
                } // faces north / south
            } else if nx > 0.0 {
                0.65 // faces east
            } else {
                0.85 // faces west
            };
            let quad = [
                (a.0, a.1, 0.0),
                (b.0, b.1, 0.0),
                (b.0, b.1, wall_top),
                (a.0, a.1, wall_top),
            ];
            let (pts, depth) = face_of(&quad, &pr);
            draws.push((
                depth,
                Draw::Fill(
                    pts,
                    rgb(shade_scene(
                        style_rgb(room.wall_rgb, self.style),
                        bright,
                        self.scene,
                    )),
                ),
            ));
        }
        // Skirting boards along every wall base.
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            let (pts, depth) = face_of(&roomforge::view3d::skirting_quad(a, b), &pr);
            draws.push((
                depth,
                Draw::Fill(
                    pts,
                    rgb(shade_scene(
                        style_rgb(room.wall_rgb, self.style),
                        1.18,
                        self.scene,
                    )),
                ),
            ));
        }
        // Soft shadows under the furniture.
        for it in &self.project.items {
            if let Some(q) = roomforge::view3d::shadow_quad(it) {
                let (pts, depth) = face_of(&q, &pr);
                draws.push((
                    depth - 0.04,
                    Draw::Fill(
                        pts,
                        rgb(shade_scene(
                            style_rgb(room.floor_rgb, self.style),
                            0.55,
                            self.scene,
                        )),
                    ),
                ));
            }
        }
        // Furniture: imported models as real geometry, the rest boxes
        for it in &self.project.items {
            if let Some(prep) = roomforge::model::model_id_of(&it.catalog_id)
                .and_then(|id| self.model_cache.get(&id))
            {
                for (tri, bright, mrgb) in roomforge::view3d::mesh_tris(it, prep) {
                    let mut pts = Vec::with_capacity(3);
                    let mut depth = 0.0;
                    for &p in &tri {
                        let (pos, d) = pr(p);
                        pts.push(pos);
                        depth += d;
                    }
                    let base = shade_scene(style_rgb(mrgb, self.style), bright, self.scene);
                    let col = if self.selected_item == Some(it.id) {
                        // Selected models blush red (boxes redden their
                        // top face; a model has no single top face).
                        Color32::from_rgb(
                            ((base[0] as u16 + 229) / 2) as u8,
                            ((base[1] as u16 + 72) / 2) as u8,
                            ((base[2] as u16 + 77) / 2) as u8,
                        )
                    } else {
                        rgb(base)
                    };
                    draws.push((depth / 3.0, Draw::Fill(pts, col)));
                }
                continue;
            }
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
                    rgb(shade_scene(
                        style_rgb(it.rgb, self.style),
                        bright,
                        self.scene,
                    ))
                };
                draws.push((depth, Draw::Fill(pts, col)));
            }
        }
        draws.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, draw) in draws {
            match draw {
                Draw::Fill(pts, col) => {
                    if pts.len() >= 3 {
                        painter.add(Shape::convex_polygon(
                            pts,
                            col,
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 60)),
                        ));
                    }
                }
                Draw::Line(pts, col) => {
                    painter.line_segment(pts, Stroke::new(1.0_f32, col));
                }
            }
        }
        // Windows glow on top of the walls they sit in; a wall cut
        // clips the glass, and hides windows above the cut entirely.
        for o in &self.project.openings {
            if o.kind != OpeningKind::Window || o.sill_m >= wall_top {
                continue;
            }
            let quad = wall_opening_quad(&self.project, o).map(|p| (p.0, p.1, p.2.min(wall_top)));
            let (pts, _) = face_of(&quad, &pr);
            painter.add(Shape::convex_polygon(
                pts,
                {
                    let g = self.scene.glass();
                    Color32::from_rgba_unmultiplied(g[0], g[1], g[2], self.scene.glass_alpha())
                },
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

    /// The item list for a multi-room plan: one section per room,
    /// then whole-home totals.
    fn list_view_multi(&mut self, ui: &mut egui::Ui) {
        let imp = self.imperial;
        ui.heading(format!(
            "Home — item list ({} rooms)",
            self.project.room_count()
        ));
        ui.label(format!(
            "{} pieces of furniture across the whole plan. No prices are tracked in RoomForge.",
            self.project.total_items()
        ));
        let mut totals = roomforge::model::MaterialsReport::default();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, name) in self.project.room_list() {
                let Some(single) = self.project.as_single(id) else {
                    continue;
                };
                ui.separator();
                ui.heading(format!(
                    "{} — {} × {}, floor area {}",
                    name,
                    fmt_len(single.room.width_m, imp),
                    fmt_len(single.room.depth_m, imp),
                    fmt_area(single.room.area_m2(), imp)
                ));
                egui::Grid::new(format!("items-{id}"))
                    .num_columns(4)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Item");
                        ui.strong("Category");
                        ui.strong("Size (W × D × H)");
                        ui.strong("Qty");
                        ui.end_row();
                        for (n, cat, w, d, h, qty) in single.item_list() {
                            ui.label(n);
                            ui.label(cat);
                            ui.label(format!(
                                "{} × {} × {}",
                                fmt_len(w, imp),
                                fmt_len(d, imp),
                                fmt_len(h, imp)
                            ));
                            ui.label(qty.to_string());
                            ui.end_row();
                        }
                    });
                for o in &single.openings {
                    let link = o
                        .links_to
                        .and_then(|t| self.project.room_name(t))
                        .map(|n| format!(" → {n}"))
                        .unwrap_or_default();
                    ui.label(format!(
                        "{} — {} wall, {} from the corner, {} wide{}",
                        o.kind.label(),
                        o.wall.label(),
                        fmt_len(o.offset_m, imp),
                        fmt_len(o.width_m, imp),
                        link
                    ));
                }
                for s in &single.services {
                    ui.label(format!(
                        "{} — {} wall, {} from the corner",
                        s.kind.label(),
                        s.wall.label(),
                        fmt_len(s.offset_m, imp)
                    ));
                }
                let m = single.materials();
                totals.floor_area_m2 += m.floor_area_m2;
                totals.ceiling_area_m2 += m.ceiling_area_m2;
                totals.wall_gross_m2 += m.wall_gross_m2;
                totals.openings_m2 += m.openings_m2;
                totals.wall_net_m2 += m.wall_net_m2;
                totals.skirting_m += m.skirting_m;
                totals.perimeter_m += m.perimeter_m;
                ui.label(format!(
                    "Materials — floor {} · walls to finish {} · skirting {}",
                    fmt_area(m.floor_area_m2, imp),
                    fmt_area(m.wall_net_m2, imp),
                    fmt_len(m.skirting_m, imp)
                ));
                let w = single.warnings();
                if !w.is_empty() {
                    ui.colored_label(RED, format!("⚠ {}", w.join(" · ")));
                }
            }
            ui.separator();
            ui.heading("Whole home — materials totals");
            ui.label(format!(
                "Floor / ceiling area {} · Wall area to finish (net) {} · Skirting {} · Perimeter {}",
                fmt_area(totals.floor_area_m2, imp),
                fmt_area(totals.wall_net_m2, imp),
                fmt_len(totals.skirting_m, imp),
                fmt_len(totals.perimeter_m, imp)
            ));
        });
    }

    fn list_view(&mut self, ui: &mut egui::Ui) {
        if self.project.room_count() > 1 {
            self.list_view_multi(ui);
            return;
        }
        ui.heading(format!("{} — item list", self.project.room.name));
        ui.label(format!(
            "Room {} × {}, ceiling {}, floor area {}. No prices are tracked in RoomForge.",
            fmt_len(self.project.room.width_m, self.imperial),
            fmt_len(self.project.room.depth_m, self.imperial),
            fmt_len(self.project.room.height_m, self.imperial),
            fmt_area(self.project.room.area_m2(), self.imperial)
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
                        ui.label(format!(
                            "{} × {} × {}",
                            fmt_len(w, self.imperial),
                            fmt_len(d, self.imperial),
                            fmt_len(h, self.imperial)
                        ));
                        ui.label(n.to_string());
                        ui.end_row();
                    }
                });
            ui.separator();
            ui.strong("Doors, windows and services");
            for o in &self.project.openings {
                ui.label(format!(
                    "{} — {} wall, {} from the corner, {} wide",
                    o.kind.label(),
                    o.wall.label(),
                    fmt_len(o.offset_m, self.imperial),
                    fmt_len(o.width_m, self.imperial)
                ));
            }
            for s in &self.project.services {
                ui.label(format!(
                    "{} — {} wall, {} from the corner",
                    s.kind.label(),
                    s.wall.label(),
                    fmt_len(s.offset_m, self.imperial)
                ));
            }
            if !self.project.annotations.is_empty() {
                ui.separator();
                ui.strong("Notes on the plan");
                for (i, a) in self.project.annotations.iter().enumerate() {
                    ui.label(format!("{}. {}", i + 1, a.text));
                }
            }
            ui.separator();
            ui.strong("Materials — quantities only (no prices, by design)");
            let m = self.project.materials();
            egui::Grid::new("materials")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Floor / ceiling area");
                    ui.label(fmt_area(m.floor_area_m2, self.imperial));
                    ui.end_row();
                    ui.label("Wall area (gross)");
                    ui.label(fmt_area(m.wall_gross_m2, self.imperial));
                    ui.end_row();
                    ui.label("Less doors & windows");
                    ui.label(fmt_area(m.openings_m2, self.imperial));
                    ui.end_row();
                    ui.label("Wall area to finish (net)");
                    ui.label(fmt_area(m.wall_net_m2, self.imperial));
                    ui.end_row();
                    ui.label("Skirting / baseboard");
                    ui.label(fmt_len(m.skirting_m, self.imperial));
                    ui.end_row();
                    ui.label("Room perimeter");
                    ui.label(fmt_len(m.perimeter_m, self.imperial));
                    ui.end_row();
                });
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

/// Draw a tiny top-down picture of a catalog item into `rect`:
/// its real footprint in its own colour, with a simple detail per
/// category (sofa arms, a bed's pillow band, a wardrobe's doors…)
/// so the catalog reads like a shelf of little plans, not a list.
fn draw_item_glyph(painter: &egui::Painter, rect: Rect, c: &CatalogItem) {
    let max_w = rect.width() - 8.0;
    let max_h = rect.height() - 8.0;
    let sc = (max_w / c.width_m as f32).min(max_h / c.depth_m as f32);
    let w = c.width_m as f32 * sc;
    let h = c.depth_m as f32 * sc;
    let fp = Rect::from_center_size(rect.center(), egui::vec2(w, h));
    let base = rgb(c.rgb);
    painter.rect_filled(fp, 2.0, base);
    painter.rect_stroke(
        fp,
        2.0,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0, 0, 0, 120)),
        egui::StrokeKind::Inside,
    );
    let ink = Color32::from_rgba_unmultiplied(0, 0, 0, 110);
    let paper = Color32::from_rgba_unmultiplied(255, 255, 255, 150);
    let x0 = fp.left();
    let x1 = fp.right();
    let y0 = fp.top();
    let y1 = fp.bottom();
    let cx = fp.center().x;
    let cy = fp.center().y;
    match c.category {
        "Seating" => {
            // Back band along the top, arm strips at the sides.
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y0 + h * 0.2)),
                0.0,
                ink,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x0 + w * 0.16, y1)),
                0.0,
                ink,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x1 - w * 0.16, y0), Pos2::new(x1, y1)),
                0.0,
                ink,
            );
        }
        "Beds" => {
            // Pillow band.
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y0 + h * 0.24)),
                0.0,
                paper,
            );
            painter.line_segment(
                [Pos2::new(x0, y0 + h * 0.55), Pos2::new(x1, y0 + h * 0.55)],
                Stroke::new(1.0_f32, ink),
            );
        }
        "Storage" => {
            // Door split down the middle.
            painter.line_segment(
                [Pos2::new(cx, y0), Pos2::new(cx, y1)],
                Stroke::new(1.0_f32, ink),
            );
            painter.line_segment(
                [Pos2::new(x0, cy), Pos2::new(x1, cy)],
                Stroke::new(1.0_f32, ink),
            );
        }
        "Kitchen" => {
            // Hob rings.
            for (fx, fy) in [(0.3, 0.3), (0.7, 0.3), (0.3, 0.7), (0.7, 0.7)] {
                painter.circle_stroke(
                    Pos2::new(x0 + w * fx, y0 + h * fy),
                    (w.min(h)) * 0.11,
                    Stroke::new(1.0_f32, ink),
                );
            }
        }
        "Bathroom" => {
            // Basin/bowl oval.
            painter.circle_stroke(
                Pos2::new(cx, cy),
                (w.min(h)) * 0.3,
                Stroke::new(1.2_f32, ink),
            );
        }
        "Lighting & decor" => {
            // Inset border (rugs) reads fine for the whole group.
            painter.rect_stroke(
                fp.shrink(3.0),
                0.0,
                Stroke::new(1.0_f32, ink),
                egui::StrokeKind::Inside,
            );
        }
        _ => {
            painter.line_segment(
                [Pos2::new(x0, cy), Pos2::new(x1, cy)],
                Stroke::new(1.0_f32, ink),
            );
        }
    }
}

/// One starter-room card in the template gallery.
fn template_card(ui: &mut egui::Ui, name: &str, blurb: &str, p: &Project) -> bool {
    let mut clicked = false;
    ui.vertical(|ui| {
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(216.0, 132.0), egui::Sense::click());
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, Color32::from_rgb(0x08, 0x08, 0x08));
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, Color32::from_rgb(0x24, 0x24, 0x24)),
            egui::StrokeKind::Inside,
        );
        // Mini plan: the room outline with furniture footprints.
        let room = &p.room;
        let inner = rect.shrink(10.0);
        let sc = (inner.width() / room.width_m as f32).min(inner.height() / room.depth_m as f32);
        let ox = inner.left() + (inner.width() - room.width_m as f32 * sc) / 2.0;
        let oy = inner.bottom() - (inner.height() - room.depth_m as f32 * sc) / 2.0;
        let ts = |x: f64, y: f64| Pos2::new(ox + x as f32 * sc, oy - y as f32 * sc);
        painter.rect_filled(
            Rect::from_two_pos(ts(0.0, room.depth_m), ts(room.width_m, 0.0)),
            0.0,
            rgb(room.floor_rgb),
        );
        for it in &p.items {
            let cs = it.corners();
            let pts: Vec<Pos2> = cs.iter().map(|&(x, y)| ts(x, y)).collect();
            painter.add(Shape::convex_polygon(pts, rgb(it.rgb), Stroke::NONE));
        }
        painter.rect_stroke(
            Rect::from_two_pos(ts(0.0, room.depth_m), ts(room.width_m, 0.0)),
            0.0,
            Stroke::new(2.0_f32, Color32::from_rgb(230, 230, 230)),
            egui::StrokeKind::Inside,
        );
        if resp.hovered() {
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(2.0_f32, RED),
                egui::StrokeKind::Inside,
            );
        }
        if resp.clicked() {
            clicked = true;
        }
        ui.strong(name);
        ui.small(blurb);
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every starter template must load clean: no overlap or
    /// blocked-door warnings out of the box. (A v0.6.0-era starter
    /// placed the floor lamp 2.5 cm into the sofa; the fix moved it,
    /// and this test keeps any future template edit honest.)
    #[test]
    fn starter_templates_have_no_warnings() {
        for (name, p) in [
            ("living", starter_project()),
            ("bedroom", bedroom_project()),
            ("office", office_project()),
        ] {
            let w = p.warnings();
            assert!(w.is_empty(), "{name} template warns: {w:?}");
        }
    }
}
