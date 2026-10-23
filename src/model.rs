//! RoomForge project model: a room to scale, its openings and services,
//! and the furniture placed in it. All lengths are metres unless a field
//! name says otherwise. This module is pure data + rules so it can be
//! tested off-Windows with no GUI involved.

use serde::{Deserialize, Serialize};

/// Current project-file format version. Readers accept this and older
/// versions they know how to migrate (see `Project::from_json`).
pub const FORMAT_VERSION: u32 = 2;

/// Which wall of the (rectangular) room something is attached to.
/// Coordinates: origin at the room's south-west corner, +x east, +y north.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wall {
    North,
    South,
    East,
    West,
}

impl Wall {
    pub fn label(self) -> &'static str {
        match self {
            Wall::North => "North",
            Wall::South => "South",
            Wall::East => "East",
            Wall::West => "West",
        }
    }
    /// Length of this wall for the given room, in metres.
    pub fn length(self, room: &Room) -> f64 {
        match self {
            Wall::North | Wall::South => room.width_m,
            Wall::East | Wall::West => room.depth_m,
        }
    }
    /// The straight runs of this wall that actually exist, as
    /// (start, end) offsets from the wall's start corner. A rectangle
    /// has one full-length run; a corner notch splits or shortens it.
    pub fn runs(self, room: &Room) -> Vec<(f64, f64)> {
        let pts = room.outline_or_rect();
        let mut runs = Vec::new();
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            let hit = match self {
                Wall::South if a.1 == 0.0 && b.1 == 0.0 => Some((a.0.min(b.0), a.0.max(b.0))),
                Wall::North if a.1 == room.depth_m && b.1 == room.depth_m => {
                    Some((a.0.min(b.0), a.0.max(b.0)))
                }
                Wall::West if a.0 == 0.0 && b.0 == 0.0 => Some((a.1.min(b.1), a.1.max(b.1))),
                Wall::East if a.0 == room.width_m && b.0 == room.width_m => {
                    Some((a.1.min(b.1), a.1.max(b.1)))
                }
                _ => None,
            };
            if let Some(r) = hit {
                if r.1 - r.0 > 1e-9 {
                    runs.push(r);
                }
            }
        }
        runs
    }
    /// The longest existing run of this wall.
    pub fn longest_run(self, room: &Room) -> (f64, f64) {
        self.runs(room)
            .into_iter()
            .max_by(|a, b| (a.1 - a.0).partial_cmp(&(b.1 - b.0)).unwrap())
            .unwrap_or((0.0, self.length(room)))
    }
}

/// A door or window cut into a wall. `offset_m` is the distance from the
/// wall's start corner (west end for N/S walls, south end for E/W walls)
/// to the opening's start edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpeningKind {
    Door,
    Window,
}

impl OpeningKind {
    pub fn label(self) -> &'static str {
        match self {
            OpeningKind::Door => "Door",
            OpeningKind::Window => "Window",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opening {
    pub id: u64,
    pub kind: OpeningKind,
    pub wall: Wall,
    pub offset_m: f64,
    pub width_m: f64,
    /// Sill height (windows) — doors are always 0. Height of the opening.
    pub sill_m: f64,
    pub height_m: f64,
    /// Doors only: swings into the room, hinge at the start edge.
    pub swings_in: bool,
}

impl Opening {
    pub fn clamp_to_wall(&mut self, room: &Room) {
        // Keep the opening inside the longest real run of its wall
        // (a notch may have shortened or split the wall).
        let (start, end) = self.wall.longest_run(room);
        let run_len = (end - start).max(0.1);
        self.width_m = self.width_m.clamp((0.3_f64).min(run_len), run_len);
        self.offset_m = self.offset_m.clamp(start, (end - self.width_m).max(start));
        self.height_m = self.height_m.clamp(0.4, room.height_m);
        self.sill_m = self
            .sill_m
            .clamp(0.0, (room.height_m - self.height_m).max(0.0));
    }
}

/// A service point fixed to a wall: electrical socket, water pipe /
/// drain, a radiator that furniture must not cover, a light switch,
/// a TV aerial point, a network point, or a thermostat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceKind {
    Socket,
    Pipe,
    Radiator,
    Switch,
    TvPoint,
    Network,
    Thermostat,
}

impl ServiceKind {
    pub fn label(self) -> &'static str {
        match self {
            ServiceKind::Socket => "Socket",
            ServiceKind::Pipe => "Pipe / water",
            ServiceKind::Radiator => "Radiator",
            ServiceKind::Switch => "Light switch",
            ServiceKind::TvPoint => "TV point",
            ServiceKind::Network => "Network point",
            ServiceKind::Thermostat => "Thermostat",
        }
    }
    /// Floor footprint width along the wall, metres.
    pub fn width_m(self) -> f64 {
        match self {
            ServiceKind::Socket => 0.15,
            ServiceKind::Pipe => 0.2,
            ServiceKind::Radiator => 1.0,
            ServiceKind::Switch => 0.09,
            ServiceKind::TvPoint => 0.12,
            ServiceKind::Network => 0.12,
            ServiceKind::Thermostat => 0.12,
        }
    }
    /// How far it projects into the room, metres.
    pub fn depth_m(self) -> f64 {
        match self {
            ServiceKind::Socket => 0.05,
            ServiceKind::Pipe => 0.1,
            ServiceKind::Radiator => 0.18,
            ServiceKind::Switch => 0.03,
            ServiceKind::TvPoint => 0.04,
            ServiceKind::Network => 0.04,
            ServiceKind::Thermostat => 0.04,
        }
    }
    /// Plan symbol: (short text, marker colour). One source of truth
    /// for the GUI and the SVG export.
    pub fn symbol(self) -> (&'static str, [u8; 3]) {
        match self {
            ServiceKind::Socket => ("E", [246, 195, 67]),
            ServiceKind::Pipe => ("W", [59, 130, 196]),
            ServiceKind::Radiator => ("R", [196, 59, 59]),
            ServiceKind::Switch => ("S", [240, 225, 130]),
            ServiceKind::TvPoint => ("TV", [150, 200, 245]),
            ServiceKind::Network => ("N", [140, 220, 170]),
            ServiceKind::Thermostat => ("TH", [235, 160, 90]),
        }
    }
    /// Furniture standing over this kind is a real problem (heat,
    /// water access), not just untidy.
    pub fn blocks_furniture(self) -> bool {
        matches!(self, ServiceKind::Pipe | ServiceKind::Radiator)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServicePoint {
    pub id: u64,
    pub kind: ServiceKind,
    pub wall: Wall,
    /// Distance from the wall's start corner to the point's centre.
    pub offset_m: f64,
}

impl ServicePoint {
    pub fn clamp_to_wall(&mut self, room: &Room) {
        let (start, end) = self.wall.longest_run(room);
        let half = self.kind.width_m() / 2.0;
        self.offset_m = self
            .offset_m
            .clamp(start + half, (end - half).max(start + half));
    }
    /// Centre position in room coordinates (metres).
    pub fn centre(&self, room: &Room) -> (f64, f64) {
        let d = self.kind.depth_m() / 2.0;
        match self.wall {
            Wall::South => (self.offset_m, d),
            Wall::North => (self.offset_m, room.depth_m - d),
            Wall::West => (d, self.offset_m),
            Wall::East => (room.width_m - d, self.offset_m),
        }
    }
}

fn default_wall_thickness() -> f64 {
    0.1
}

/// Flooring material. Procedural — a base colour plus a pattern drawn
/// from the material kind (planks, tiles, joints, or plain) — so plans
/// stay self-contained with no texture files to lose. `Solid` keeps
/// whatever colour the user picked in `Room::floor_rgb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FloorMaterial {
    #[default]
    Solid,
    OakWood,
    WalnutWood,
    GreyLaminate,
    CeramicTile,
    SlateTile,
    BeigeCarpet,
    PolishedConcrete,
}

/// How a material's surface pattern is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloorPattern {
    None,
    /// Planks running east–west, `row` metres wide, staggered joints.
    Planks {
        row: f64,
        joint_every: f64,
    },
    /// Square tile grid, `size` metres.
    Tiles {
        size: f64,
    },
    /// Sparse expansion-joint grid, `size` metres.
    Joints {
        size: f64,
    },
}

impl FloorMaterial {
    pub fn label(self) -> &'static str {
        match self {
            FloorMaterial::Solid => "Solid colour (your pick)",
            FloorMaterial::OakWood => "Oak wood planks",
            FloorMaterial::WalnutWood => "Walnut wood planks",
            FloorMaterial::GreyLaminate => "Grey laminate planks",
            FloorMaterial::CeramicTile => "Ceramic tiles",
            FloorMaterial::SlateTile => "Slate tiles",
            FloorMaterial::BeigeCarpet => "Beige carpet",
            FloorMaterial::PolishedConcrete => "Polished concrete",
        }
    }
    pub fn all() -> [FloorMaterial; 8] {
        [
            FloorMaterial::Solid,
            FloorMaterial::OakWood,
            FloorMaterial::WalnutWood,
            FloorMaterial::GreyLaminate,
            FloorMaterial::CeramicTile,
            FloorMaterial::SlateTile,
            FloorMaterial::BeigeCarpet,
            FloorMaterial::PolishedConcrete,
        ]
    }
    /// The material's natural base colour, or None for Solid (the
    /// user's own colour stands).
    pub fn base_rgb(self) -> Option<[u8; 3]> {
        match self {
            FloorMaterial::Solid => None,
            FloorMaterial::OakWood => Some([176, 132, 84]),
            FloorMaterial::WalnutWood => Some([110, 76, 52]),
            FloorMaterial::GreyLaminate => Some([150, 148, 144]),
            FloorMaterial::CeramicTile => Some([206, 201, 191]),
            FloorMaterial::SlateTile => Some([104, 110, 116]),
            FloorMaterial::BeigeCarpet => Some([188, 170, 148]),
            FloorMaterial::PolishedConcrete => Some([141, 141, 139]),
        }
    }
    pub fn pattern(self) -> FloorPattern {
        match self {
            FloorMaterial::Solid | FloorMaterial::BeigeCarpet => FloorPattern::None,
            FloorMaterial::OakWood | FloorMaterial::WalnutWood | FloorMaterial::GreyLaminate => {
                FloorPattern::Planks {
                    row: 0.12,
                    joint_every: 1.8,
                }
            }
            FloorMaterial::CeramicTile => FloorPattern::Tiles { size: 0.3 },
            FloorMaterial::SlateTile => FloorPattern::Tiles { size: 0.45 },
            FloorMaterial::PolishedConcrete => FloorPattern::Joints { size: 1.0 },
        }
    }
    /// Pattern line segments (room coordinates) for the given floor
    /// cells — one source of truth for the 2D plan, the 3D view, the
    /// SVG export and the PNG snapshot. Lines align to the room's
    /// global grid, so they run continuously across cells.
    pub fn pattern_segments(self, cells: &[(f64, f64, f64, f64)]) -> Vec<((f64, f64), (f64, f64))> {
        let mut out = Vec::new();
        let eps = 1e-9;
        match self.pattern() {
            FloorPattern::None => {}
            FloorPattern::Tiles { size } | FloorPattern::Joints { size } => {
                for &(x1, y1, x2, y2) in cells {
                    let mut gx = (x1 / size).floor() * size + size;
                    while gx < x2 - eps {
                        out.push(((gx, y1), (gx, y2)));
                        gx += size;
                    }
                    let mut gy = (y1 / size).floor() * size + size;
                    while gy < y2 - eps {
                        out.push(((x1, gy), (x2, gy)));
                        gy += size;
                    }
                }
            }
            FloorPattern::Planks { row, joint_every } => {
                for &(x1, y1, x2, y2) in cells {
                    let first_row = (y1 / row).floor() as i64;
                    let mut gy = first_row as f64 * row + row;
                    while gy < y2 - eps {
                        out.push(((x1, gy), (x2, gy)));
                        gy += row;
                    }
                    // Staggered butt joints within each row band the
                    // cell crosses.
                    let mut band = first_row;
                    loop {
                        let by1 = band as f64 * row;
                        let by2 = by1 + row;
                        if by1 >= y2 - eps {
                            break;
                        }
                        let offset = ((band * 7) % 5) as f64 * (joint_every / 5.0);
                        let mut k = ((x1 - offset) / joint_every).ceil() as i64;
                        loop {
                            let jx = offset + k as f64 * joint_every;
                            if jx >= x2 - eps {
                                break;
                            }
                            if jx > x1 + eps {
                                out.push(((jx, by1.max(y1)), (jx, by2.min(y2))));
                            }
                            k += 1;
                        }
                        band += 1;
                    }
                }
            }
        }
        out
    }
}

/// A corner of the room's bounding rectangle, for notch cutting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Corner {
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl Corner {
    pub fn label(self) -> &'static str {
        match self {
            Corner::NorthEast => "North-east",
            Corner::NorthWest => "North-west",
            Corner::SouthEast => "South-east",
            Corner::SouthWest => "South-west",
        }
    }
    /// The bounding-box point of this corner.
    pub fn point(self, room: &Room) -> (f64, f64) {
        match self {
            Corner::NorthEast => (room.width_m, room.depth_m),
            Corner::NorthWest => (0.0, room.depth_m),
            Corner::SouthEast => (room.width_m, 0.0),
            Corner::SouthWest => (0.0, 0.0),
        }
    }
}

/// The room shell: a bounding width (W–E) and depth (S–N), a ceiling
/// height, and — since format v2 — an optional rectilinear outline.
/// An empty outline means the plain rectangle; cutting a corner notch
/// produces an L (or, with more notches, a U/C) shape. Openings and
/// services still attach to the four outer walls by offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub width_m: f64,
    pub depth_m: f64,
    pub height_m: f64,
    /// Wall thickness in metres (drawn on the plan; structural in
    /// spirit — the room dimensions stay internal clear dimensions).
    #[serde(default = "default_wall_thickness")]
    pub wall_thickness_m: f64,
    /// Outline vertices (CCW, room coordinates). Empty = rectangle.
    #[serde(default)]
    pub outline: Vec<(f64, f64)>,
    /// Flooring material (procedural pattern over `floor_rgb`).
    #[serde(default)]
    pub floor_material: FloorMaterial,
    /// Wall colour as [r, g, b] 0–255.
    pub wall_rgb: [u8; 3],
    /// Floor colour as [r, g, b] 0–255.
    pub floor_rgb: [u8; 3],
}

impl Room {
    /// The effective outline: the stored one, or the rectangle.
    pub fn outline_or_rect(&self) -> Vec<(f64, f64)> {
        if self.outline.len() >= 4 {
            self.outline.clone()
        } else {
            vec![
                (0.0, 0.0),
                (self.width_m, 0.0),
                (self.width_m, self.depth_m),
                (0.0, self.depth_m),
            ]
        }
    }
    pub fn area_m2(&self) -> f64 {
        crate::geometry::polygon_area(&self.outline_or_rect())
    }
    pub fn perimeter_m(&self) -> f64 {
        crate::geometry::polygon_perimeter(&self.outline_or_rect())
    }
    pub fn contains(&self, x: f64, y: f64) -> bool {
        crate::geometry::point_in_polygon(x, y, &self.outline_or_rect())
    }
    /// Bring a point inside the room: unchanged when already inside,
    /// otherwise the nearest boundary point nudged slightly inward.
    pub fn clamp_point_inside(&self, x: f64, y: f64) -> (f64, f64) {
        let x = x.clamp(0.0, self.width_m);
        let y = y.clamp(0.0, self.depth_m);
        if self.contains(x, y) {
            return (x, y);
        }
        let pts = self.outline_or_rect();
        let mut best = (x, y);
        let mut best_d = f64::INFINITY;
        for i in 0..pts.len() {
            let (x1, y1) = pts[i];
            let (x2, y2) = pts[(i + 1) % pts.len()];
            let (dx, dy) = (x2 - x1, y2 - y1);
            let len2 = dx * dx + dy * dy;
            if len2 <= 0.0 {
                continue;
            }
            let t = (((x - x1) * dx + (y - y1) * dy) / len2).clamp(0.0, 1.0);
            let (cx, cy) = (x1 + t * dx, y1 + t * dy);
            let d = (x - cx).powi(2) + (y - cy).powi(2);
            if d < best_d {
                best_d = d;
                best = (cx, cy);
            }
        }
        let (gx, gy) = crate::geometry::polygon_centroid(&pts);
        let (vx, vy) = (gx - best.0, gy - best.1);
        let vl = (vx * vx + vy * vy).sqrt();
        if vl > 1e-9 {
            best = (best.0 + vx / vl * 0.01, best.1 + vy / vl * 0.01);
        }
        best
    }
    pub fn corner_is_cut(&self, corner: Corner) -> bool {
        let cp = corner.point(self);
        !self
            .outline_or_rect()
            .iter()
            .any(|p| (p.0 - cp.0).abs() < 1e-9 && (p.1 - cp.1).abs() < 1e-9)
    }
    /// Cut a rectangular notch out of a bounding-box corner.
    /// `along_h` is the notch's extent along the horizontal wall,
    /// `along_v` along the vertical wall. Fails when the corner is
    /// already cut or the notch would swallow a wall (each cut must
    /// be ≥ 0.3 m and leave ≥ 0.3 m of each adjacent wall run).
    pub fn cut_notch(&mut self, corner: Corner, along_h: f64, along_v: f64) -> Result<(), String> {
        if along_h < 0.3 || along_v < 0.3 {
            return Err("A notch must be at least 0.3 m along each wall.".into());
        }
        let mut pts = self.outline_or_rect();
        let cp = corner.point(self);
        let idx = pts
            .iter()
            .position(|p| (p.0 - cp.0).abs() < 1e-9 && (p.1 - cp.1).abs() < 1e-9)
            .ok_or_else(|| format!("The {} corner is already cut.", corner.label()))?;
        let n = pts.len();
        let prev = pts[(idx + n - 1) % n];
        let next = pts[(idx + 1) % n];
        // Distance to walk from the corner toward prev / next, based on
        // whether that edge is horizontal or vertical.
        let toward = |from: (f64, f64), to: (f64, f64), dist: f64| -> (f64, f64) {
            let len = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
            (
                from.0 + (to.0 - from.0) / len * dist,
                from.1 + (to.1 - from.1) / len * dist,
            )
        };
        let dist =
            |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let d_prev = if (prev.1 - cp.1).abs() < 1e-9 {
            along_h
        } else {
            along_v
        };
        let d_next = if (next.1 - cp.1).abs() < 1e-9 {
            along_h
        } else {
            along_v
        };
        if dist(cp, prev) < d_prev + 0.3 || dist(cp, next) < d_next + 0.3 {
            return Err(format!(
                "That notch is too big for the {} corner — it would swallow the wall. Make it smaller, or enlarge the room first.",
                corner.label()
            ));
        }
        let a = toward(cp, prev, d_prev);
        let b = toward(cp, next, d_next);
        // The notch's inner corner: corner + both cut offsets.
        let inner = (a.0 + b.0 - cp.0, a.1 + b.1 - cp.1);
        pts.splice(idx..=idx, [a, inner, b]);
        self.outline = pts;
        Ok(())
    }
    /// Undo all notches: back to the plain rectangle.
    pub fn clear_outline(&mut self) {
        self.outline.clear();
    }
    pub fn shape_label(&self) -> String {
        match self.outline_or_rect().len() {
            4 => "Rectangle".to_string(),
            6 => "L-shape (one corner notch)".to_string(),
            8 => "Two corner notches".to_string(),
            10 => "Three corner notches".to_string(),
            12 => "Four corner notches".to_string(),
            n => format!("Custom outline ({n} corners)"),
        }
    }
    pub fn clamp_dims(&mut self) {
        self.width_m = self.width_m.clamp(1.0, 30.0);
        self.depth_m = self.depth_m.clamp(1.0, 30.0);
        self.height_m = self.height_m.clamp(1.8, 6.0);
        self.wall_thickness_m = self.wall_thickness_m.clamp(0.05, 0.6);
        // An outline that no longer fits the bounding box is dropped.
        if self.outline.iter().any(|p| {
            p.0 < -1e-9 || p.1 < -1e-9 || p.0 > self.width_m + 1e-9 || p.1 > self.depth_m + 1e-9
        }) {
            self.outline.clear();
        }
    }
}

impl Default for Room {
    fn default() -> Self {
        Room {
            name: "Living room".to_string(),
            width_m: 5.0,
            depth_m: 4.0,
            height_m: 2.5,
            wall_thickness_m: default_wall_thickness(),
            outline: Vec::new(),
            floor_material: FloorMaterial::Solid,
            wall_rgb: [216, 210, 200],
            floor_rgb: [148, 112, 76],
        }
    }
}

/// A free-text note pinned to a point on the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: u64,
    pub x_m: f64,
    pub y_m: f64,
    pub text: String,
}

impl Annotation {
    pub fn clamp_inside(&mut self, room: &Room) {
        let (x, y) = room.clamp_point_inside(self.x_m, self.y_m);
        self.x_m = x;
        self.y_m = y;
    }
}

/// One placed piece of furniture. `x_m`/`y_m` are the centre in room
/// coordinates; `rotation_deg` rotates around the centre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedItem {
    pub id: u64,
    /// Catalog id this was placed from, or "custom".
    pub catalog_id: String,
    pub name: String,
    pub category: String,
    pub x_m: f64,
    pub y_m: f64,
    pub width_m: f64,
    pub depth_m: f64,
    pub height_m: f64,
    pub rotation_deg: f64,
    pub rgb: [u8; 3],
}

impl PlacedItem {
    /// The four corners in room coordinates, after rotation.
    pub fn corners(&self) -> [(f64, f64); 4] {
        let (hw, hd) = (self.width_m / 2.0, self.depth_m / 2.0);
        let rad = self.rotation_deg.to_radians();
        let (s, c) = rad.sin_cos();
        let local = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
        local.map(|(lx, ly)| (self.x_m + lx * c - ly * s, self.y_m + lx * s + ly * c))
    }
    /// Keep the item's centre inside the room outline (with a small
    /// margin so a rotated item can still be grabbed at the edges).
    pub fn clamp_inside(&mut self, room: &Room) {
        let (x, y) = room.clamp_point_inside(self.x_m, self.y_m);
        self.x_m = x;
        self.y_m = y;
    }
    /// Clear space between this item's (rotated) bounding box and
    /// each side of the room, as (west, east, south, north) in metres,
    /// never negative. Measured to the room's bounding rectangle, so
    /// treat it as a guide in notched rooms.
    pub fn wall_clearances(&self, room: &Room) -> (f64, f64, f64, f64) {
        let cs = self.corners();
        let min_x = cs.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
        let max_x = cs.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
        let min_y = cs.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
        let max_y = cs.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);
        (
            min_x.max(0.0),
            (room.width_m - max_x).max(0.0),
            min_y.max(0.0),
            (room.depth_m - max_y).max(0.0),
        )
    }
    pub fn normalised_rotation(&self) -> f64 {
        let r = self.rotation_deg % 360.0;
        if r < 0.0 {
            r + 360.0
        } else {
            r
        }
    }
}

/// A floor-plan image kept under the drawing for tracing (a scanned
/// or photographed plan). The original image bytes are embedded
/// base64 so a plan file stays self-contained and portable. The
/// mapping is rotation-free (plans are axis-aligned): `origin` is the
/// room position of the image's top-left corner; image y grows
/// downward, room y grows north, hence the minus in the transform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Underlay {
    /// Original file bytes (PNG/JPEG/BMP), base64-encoded.
    pub data_b64: String,
    pub width_px: u32,
    pub height_px: u32,
    pub origin_x_m: f64,
    pub origin_y_m: f64,
    pub m_per_px: f64,
    pub opacity: f32,
}

impl Underlay {
    /// Fit the image exactly over the room.
    pub fn fit(room: &Room, width_px: u32, height_px: u32, data_b64: String) -> Underlay {
        Underlay {
            data_b64,
            width_px,
            height_px,
            origin_x_m: 0.0,
            origin_y_m: room.depth_m,
            m_per_px: room.width_m / width_px.max(1) as f64,
            opacity: 0.55,
        }
    }
    /// Image pixel → room coordinates.
    pub fn room_from_px(&self, px: f64, py: f64) -> (f64, f64) {
        (
            self.origin_x_m + px * self.m_per_px,
            self.origin_y_m - py * self.m_per_px,
        )
    }
    /// Room coordinates → image pixel.
    pub fn px_from_room(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.origin_x_m) / self.m_per_px,
            (self.origin_y_m - y) / self.m_per_px,
        )
    }
    /// Two-point calibration: the user marked two image points and
    /// states the real distance between them. The first point keeps
    /// its current room position; the scale is recomputed.
    pub fn calibrate(&mut self, a_px: (f64, f64), b_px: (f64, f64), real_distance_m: f64) {
        let d_px = ((b_px.0 - a_px.0).powi(2) + (b_px.1 - a_px.1).powi(2)).sqrt();
        if d_px < 1.0 || real_distance_m <= 0.0 {
            return;
        }
        let anchor_room = self.room_from_px(a_px.0, a_px.1);
        self.m_per_px = real_distance_m / d_px;
        self.origin_x_m = anchor_room.0 - a_px.0 * self.m_per_px;
        self.origin_y_m = anchor_room.1 + a_px.1 * self.m_per_px;
    }
}

/// A furniture definition the user created themselves (the
/// custom-object builder). Stored in the project so plans stay
/// portable; placed instances reference it by `custom-<id>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomItemDef {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub width_m: f64,
    pub depth_m: f64,
    pub height_m: f64,
    pub rgb: [u8; 3],
}

impl CustomItemDef {
    pub fn clamp_dims(&mut self) {
        self.width_m = self.width_m.clamp(0.05, 10.0);
        self.depth_m = self.depth_m.clamp(0.05, 10.0);
        self.height_m = self.height_m.clamp(0.05, 5.0);
        if self.name.trim().is_empty() {
            self.name = "Custom item".to_string();
        }
        if self.category.trim().is_empty() {
            self.category = "Other".to_string();
        }
    }
    /// The catalog id placed instances of this definition carry.
    pub fn catalog_id(&self) -> String {
        format!("custom-{}", self.id)
    }
}

/// Quantities for buying materials — areas and lengths only.
/// There are deliberately no prices or costs anywhere in RoomForge.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialsReport {
    pub floor_area_m2: f64,
    pub ceiling_area_m2: f64,
    pub wall_gross_m2: f64,
    pub openings_m2: f64,
    pub wall_net_m2: f64,
    pub skirting_m: f64,
    pub perimeter_m: f64,
}

/// A whole design: one room, its openings/services, and its furniture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub room: Room,
    pub openings: Vec<Opening>,
    pub services: Vec<ServicePoint>,
    pub items: Vec<PlacedItem>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    #[serde(default)]
    pub underlay: Option<Underlay>,
    #[serde(default)]
    pub custom_items: Vec<CustomItemDef>,
    /// Next id to hand out. Persisted so ids never repeat after a reload.
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            format_version: FORMAT_VERSION,
            room: Room::default(),
            openings: Vec::new(),
            services: Vec::new(),
            items: Vec::new(),
            annotations: Vec::new(),
            underlay: None,
            custom_items: Vec::new(),
            next_id: 1,
        }
    }
}

impl Project {
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse a project file. Rejects files from a newer format rather
    /// than silently misreading them.
    pub fn from_json(text: &str) -> Result<Project, String> {
        let mut p: Project =
            serde_json::from_str(text).map_err(|e| format!("This file could not be read: {e}"))?;
        if p.format_version > FORMAT_VERSION {
            return Err(format!(
                "This plan was saved by a newer version of RoomForge (format {}). Update RoomForge to open it.",
                p.format_version
            ));
        }
        p.format_version = FORMAT_VERSION;
        // Repair any hand-edited or legacy values instead of trusting them.
        p.room.clamp_dims();
        let room = p.room.clone();
        for o in &mut p.openings {
            o.clamp_to_wall(&room);
        }
        for s in &mut p.services {
            s.clamp_to_wall(&room);
        }
        for a in &mut p.annotations {
            a.clamp_inside(&room);
        }
        for c in &mut p.custom_items {
            c.clamp_dims();
        }
        let max_id = p
            .items
            .iter()
            .map(|i| i.id)
            .chain(p.openings.iter().map(|o| o.id))
            .chain(p.services.iter().map(|s| s.id))
            .chain(p.annotations.iter().map(|a| a.id))
            .chain(p.custom_items.iter().map(|c| c.id))
            .max()
            .unwrap_or(0);
        if p.next_id <= max_id {
            p.next_id = max_id + 1;
        }
        Ok(p)
    }

    /// Grouped item list for display/export: (name, category, w, d, h, count),
    /// sorted by category then name. Dimensions are the placed size, so a
    /// resized item lists separately from a standard one.
    pub fn item_list(&self) -> Vec<(String, String, f64, f64, f64, u32)> {
        let mut rows: Vec<(String, String, f64, f64, f64, u32)> = Vec::new();
        for it in &self.items {
            let key = rows.iter_mut().find(|r| {
                r.0 == it.name
                    && r.1 == it.category
                    && (r.2 - it.width_m).abs() < 0.001
                    && (r.3 - it.depth_m).abs() < 0.001
                    && (r.4 - it.height_m).abs() < 0.001
            });
            match key {
                Some(r) => r.5 += 1,
                None => rows.push((
                    it.name.clone(),
                    it.category.clone(),
                    it.width_m,
                    it.depth_m,
                    it.height_m,
                    1,
                )),
            }
        }
        rows.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        rows
    }

    /// Material quantities for the room shell: floor/ceiling area,
    /// wall area before and after subtracting doors and windows, and
    /// skirting length (perimeter minus door widths). Quantities
    /// only — never prices.
    pub fn materials(&self) -> MaterialsReport {
        let r = &self.room;
        let perimeter = r.perimeter_m();
        let wall_gross = perimeter * r.height_m;
        let openings: f64 = self.openings.iter().map(|o| o.width_m * o.height_m).sum();
        let door_widths: f64 = self
            .openings
            .iter()
            .filter(|o| o.kind == OpeningKind::Door)
            .map(|o| o.width_m)
            .sum();
        MaterialsReport {
            floor_area_m2: r.area_m2(),
            ceiling_area_m2: r.area_m2(),
            wall_gross_m2: wall_gross,
            openings_m2: openings,
            wall_net_m2: (wall_gross - openings).max(0.0),
            skirting_m: (perimeter - door_widths).max(0.0),
            perimeter_m: perimeter,
        }
    }

    /// Human-readable warnings: furniture overlaps and items covering a
    /// radiator or pipe. Doors swinging into furniture is also reported.
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for i in 0..self.items.len() {
            for j in (i + 1)..self.items.len() {
                // Flat items (rugs, 0.03 m or thinner) are meant to go
                // under furniture — never warn about those pairs.
                if self.items[i].height_m <= 0.03 || self.items[j].height_m <= 0.03 {
                    continue;
                }
                if crate::geometry::obb_overlap(&self.items[i].corners(), &self.items[j].corners())
                {
                    out.push(format!(
                        "{} overlaps {}",
                        self.items[i].name, self.items[j].name
                    ));
                }
            }
        }
        for s in &self.services {
            if !s.kind.blocks_furniture() {
                continue; // flat wall points may legitimately sit behind furniture
            }
            let (sx, sy) = s.centre(&self.room);
            for it in &self.items {
                if crate::geometry::point_in_obb(sx, sy, &it.corners()) {
                    out.push(format!(
                        "{} covers the {}",
                        it.name,
                        s.kind.label().to_lowercase()
                    ));
                }
            }
        }
        for o in &self.openings {
            if let Some(sector) = door_swing_sector(o, &self.room) {
                for it in &self.items {
                    let corners: Vec<(f64, f64)> = it.corners().to_vec();
                    if crate::geometry::convex_overlap(&sector, &corners) {
                        out.push(format!(
                            "{} blocks the {} door's swing",
                            it.name,
                            o.wall.label().to_lowercase()
                        ));
                    }
                }
            }
        }
        out
    }
}

/// The floor area a door sweeps as it opens, as a convex polygon
/// (hinge point + arc samples). Only inward-swinging doors can hit
/// furniture — items are clamped inside the room — so outward doors
/// return None. The hinge is at the opening's start edge.
pub fn door_swing_sector(o: &Opening, room: &Room) -> Option<Vec<(f64, f64)>> {
    if o.kind != OpeningKind::Door || !o.swings_in {
        return None;
    }
    let r = o.width_m;
    let (hx, hy, a0, a1) = match o.wall {
        // Hinge on the south wall: leaf closes along +x, opens to +y.
        Wall::South => (o.offset_m, 0.0, 0.0, 90.0),
        // North wall: closes along +x, opens to -y.
        Wall::North => (o.offset_m, room.depth_m, -90.0, 0.0),
        // West wall: closes along +y, opens to +x.
        Wall::West => (0.0, o.offset_m, 0.0, 90.0),
        // East wall: closes along +y, opens to -x.
        Wall::East => (room.width_m, o.offset_m, 90.0, 180.0),
    };
    let mut poly = vec![(hx, hy)];
    let steps = 6;
    for i in 0..=steps {
        let a = (a0 + (a1 - a0) * (i as f64) / (steps as f64)).to_radians();
        poly.push((hx + r * a.cos(), hy + r * a.sin()));
    }
    Some(poly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_preserves_project() {
        let mut p = Project::default();
        let id = p.alloc_id();
        p.items.push(PlacedItem {
            id,
            catalog_id: "sofa-3".into(),
            name: "3-seat sofa".into(),
            category: "Seating".into(),
            x_m: 2.0,
            y_m: 1.0,
            width_m: 2.2,
            depth_m: 0.95,
            height_m: 0.85,
            rotation_deg: 90.0,
            rgb: [90, 90, 100],
        });
        let text = p.to_json().unwrap();
        let back = Project::from_json(&text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn newer_format_is_rejected_with_a_clear_message() {
        let p = Project {
            format_version: FORMAT_VERSION + 1,
            ..Project::default()
        };
        let text = p.to_json().unwrap();
        let err = Project::from_json(&text).unwrap_err();
        assert!(err.contains("newer version"), "got: {err}");
    }

    #[test]
    fn item_list_groups_identical_items() {
        let mut p = Project::default();
        for x in [1.0, 2.0] {
            let id = p.alloc_id();
            p.items.push(PlacedItem {
                id,
                catalog_id: "chair".into(),
                name: "Dining chair".into(),
                category: "Seating".into(),
                x_m: x,
                y_m: 1.0,
                width_m: 0.45,
                depth_m: 0.5,
                height_m: 0.9,
                rotation_deg: 0.0,
                rgb: [1, 2, 3],
            });
        }
        let rows = p.item_list();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].5, 2);
    }

    #[test]
    fn overlapping_items_produce_a_warning() {
        let mut p = Project::default();
        for _ in 0..2 {
            let id = p.alloc_id();
            p.items.push(PlacedItem {
                id,
                catalog_id: "table".into(),
                name: "Table".into(),
                category: "Tables".into(),
                x_m: 2.0,
                y_m: 2.0,
                width_m: 1.0,
                depth_m: 1.0,
                height_m: 0.75,
                rotation_deg: 0.0,
                rgb: [0, 0, 0],
            });
        }
        assert_eq!(p.warnings().len(), 1);
    }

    fn door_and_item(item_x: f64, item_y: f64) -> Project {
        let mut p = Project::default(); // 5 x 4 room
        let did = p.alloc_id();
        p.openings.push(Opening {
            id: did,
            kind: OpeningKind::Door,
            wall: Wall::South,
            offset_m: 1.0,
            width_m: 0.9,
            sill_m: 0.0,
            height_m: 2.1,
            swings_in: true,
        });
        let iid = p.alloc_id();
        p.items.push(PlacedItem {
            id: iid,
            catalog_id: "chair".into(),
            name: "Chair".into(),
            category: "Seating".into(),
            x_m: item_x,
            y_m: item_y,
            width_m: 0.45,
            depth_m: 0.45,
            height_m: 0.9,
            rotation_deg: 0.0,
            rgb: [0, 0, 0],
        });
        p
    }

    #[test]
    fn door_swing_blocked_and_clear() {
        // Chair right in the swing path of the south door (hinge x=1.0).
        let p = door_and_item(1.5, 0.5);
        assert!(
            p.warnings().iter().any(|w| w.contains("swing")),
            "warnings: {:?}",
            p.warnings()
        );
        // Chair far from the door: no swing warning.
        let p = door_and_item(4.0, 3.0);
        assert!(!p.warnings().iter().any(|w| w.contains("swing")));
    }

    #[test]
    fn wall_clearances_measure_the_gaps() {
        let p = Project::default(); // 5 x 4 room
        let mut it = PlacedItem {
            id: 1,
            catalog_id: "x".into(),
            name: "Box".into(),
            category: "Test".into(),
            x_m: 2.5,
            y_m: 2.0,
            width_m: 1.0,
            depth_m: 1.0,
            height_m: 1.0,
            rotation_deg: 0.0,
            rgb: [0, 0, 0],
        };
        let (w, e, s, n) = it.wall_clearances(&p.room);
        assert!((w - 2.0).abs() < 1e-9 && (e - 2.0).abs() < 1e-9);
        assert!((s - 1.5).abs() < 1e-9 && (n - 1.5).abs() < 1e-9);
        // Rotating 45° grows the bounding box, shrinking every gap.
        it.rotation_deg = 45.0;
        let (w2, _, s2, _) = it.wall_clearances(&p.room);
        assert!(w2 < w && s2 < s, "rotated gaps {w2} {s2}");
    }

    #[test]
    fn rugs_under_furniture_do_not_warn() {
        let mut p = Project::default();
        let mk = |p: &mut Project, name: &str, h: f64| {
            let id = p.alloc_id();
            p.items.push(PlacedItem {
                id,
                catalog_id: "x".into(),
                name: name.into(),
                category: "Test".into(),
                x_m: 2.0,
                y_m: 2.0,
                width_m: 1.5,
                depth_m: 1.0,
                height_m: h,
                rotation_deg: 0.0,
                rgb: [1, 2, 3],
            });
        };
        mk(&mut p, "Rug, large", 0.02);
        mk(&mut p, "Sofa", 0.85);
        assert!(
            p.warnings().iter().all(|w| !w.contains("overlaps")),
            "rug under a sofa must not warn: {:?}",
            p.warnings()
        );
        // Two real pieces in the same spot still warn.
        let mut p2 = Project::default();
        mk(&mut p2, "Sofa", 0.85);
        mk(&mut p2, "Table", 0.75);
        assert!(p2
            .warnings()
            .iter()
            .any(|w| w == "Sofa overlaps Table" || w == "Table overlaps Sofa"));
    }

    #[test]
    fn floor_pattern_segments_align_to_the_room_grid() {
        let cells = vec![(0.0, 0.0, 0.9, 0.9)];
        // 30 cm tiles: lines at 0.3 and 0.6 in both directions.
        let segs = FloorMaterial::CeramicTile.pattern_segments(&cells);
        assert_eq!(segs.len(), 4, "{segs:?}");
        assert!(segs
            .iter()
            .all(|&((x1, y1), (x2, y2))| x1 >= 0.0 && y1 >= 0.0 && x2 <= 0.9 && y2 <= 0.9));
        // Solid and carpet have no pattern lines.
        assert!(FloorMaterial::Solid.pattern_segments(&cells).is_empty());
        assert!(FloorMaterial::BeigeCarpet
            .pattern_segments(&cells)
            .is_empty());
        // Planks: one row line at 0.12 in a 0.24-tall strip, plus
        // staggered joints (row band 0 joints at 1.8; band 1 at 0.72).
        let strip = vec![(0.0, 0.0, 2.0, 0.24)];
        let segs = FloorMaterial::OakWood.pattern_segments(&strip);
        assert_eq!(segs.len(), 3, "{segs:?}");
        // The material survives a save/load round-trip.
        let mut p = Project::default();
        p.room.floor_material = FloorMaterial::SlateTile;
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.room.floor_material, FloorMaterial::SlateTile);
        // Old files (no floor_material key) default to Solid.
        let legacy = r#"{"format_version":2,"room":{"name":"R","width_m":5.0,
            "depth_m":4.0,"height_m":2.5,"wall_rgb":[1,2,3],"floor_rgb":[4,5,6]},
            "openings":[],"services":[],"items":[],"next_id":1}"#;
        let old = Project::from_json(legacy).unwrap();
        assert_eq!(old.room.floor_material, FloorMaterial::Solid);
    }

    #[test]
    fn notch_cutting_makes_an_l_and_clamps_things() {
        let mut p = Project::default(); // 5 x 4
        assert_eq!(p.room.shape_label(), "Rectangle");
        p.room.cut_notch(Corner::NorthEast, 2.0, 1.5).unwrap();
        assert_eq!(p.room.outline_or_rect().len(), 6);
        assert!((p.room.area_m2() - 17.0).abs() < 1e-9);
        assert!((p.room.perimeter_m() - 18.0).abs() < 1e-9);
        assert_eq!(p.room.shape_label(), "L-shape (one corner notch)");
        assert!(p.room.corner_is_cut(Corner::NorthEast));
        // The same corner again is an error; so is a giant notch.
        assert!(p.room.cut_notch(Corner::NorthEast, 1.0, 1.0).is_err());
        assert!(p.room.cut_notch(Corner::SouthWest, 4.9, 1.0).is_err());
        // A second corner works.
        p.room.cut_notch(Corner::SouthWest, 1.0, 1.0).unwrap();
        assert_eq!(p.room.outline_or_rect().len(), 8);
        // A point in the NE notch gets pulled inside.
        let (x, y) = p.room.clamp_point_inside(4.5, 3.5);
        assert!(p.room.contains(x, y), "got {x},{y}");
        // A north-wall opening beyond the remaining run is pulled in.
        let oid = p.alloc_id();
        let mut o = Opening {
            id: oid,
            kind: OpeningKind::Window,
            wall: Wall::North,
            offset_m: 4.0,
            width_m: 1.5,
            sill_m: 0.9,
            height_m: 1.2,
            swings_in: false,
        };
        o.clamp_to_wall(&p.room);
        assert!(
            o.offset_m + o.width_m <= 3.0 + 1e-9,
            "offset {}",
            o.offset_m
        );
        // Restore rectangle.
        p.room.clear_outline();
        assert!((p.room.area_m2() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn v1_files_load_as_rectangles_in_v2() {
        let legacy = r#"{"format_version":1,"room":{"name":"R","width_m":5.0,
            "depth_m":4.0,"height_m":2.5,"wall_rgb":[1,2,3],"floor_rgb":[4,5,6]},
            "openings":[],"services":[],"items":[],"next_id":1}"#;
        let p = Project::from_json(legacy).unwrap();
        assert_eq!(p.format_version, FORMAT_VERSION);
        assert!(p.room.outline.is_empty());
        assert!((p.room.area_m2() - 20.0).abs() < 1e-9);
    }

    #[test]
    fn materials_quantities() {
        let mut p = Project::default(); // 5 x 4 m, 2.5 ceiling
        let m = p.materials();
        assert!((m.floor_area_m2 - 20.0).abs() < 1e-9);
        assert!((m.perimeter_m - 18.0).abs() < 1e-9);
        assert!((m.wall_gross_m2 - 45.0).abs() < 1e-9);
        assert!((m.wall_net_m2 - 45.0).abs() < 1e-9);
        let did = p.alloc_id();
        p.openings.push(Opening {
            id: did,
            kind: OpeningKind::Door,
            wall: Wall::South,
            offset_m: 0.5,
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
            offset_m: 1.0,
            width_m: 1.5,
            sill_m: 0.9,
            height_m: 1.2,
            swings_in: false,
        });
        let m = p.materials();
        assert!((m.openings_m2 - (1.89 + 1.8)).abs() < 1e-9);
        assert!((m.wall_net_m2 - (45.0 - 3.69)).abs() < 1e-9);
        assert!((m.skirting_m - (18.0 - 0.9)).abs() < 1e-9);
    }

    #[test]
    fn custom_items_roundtrip_and_clamp() {
        let mut p = Project::default();
        let id = p.alloc_id();
        p.custom_items.push(CustomItemDef {
            id,
            name: "Aquarium stand".into(),
            category: "Storage".into(),
            width_m: 99.0,
            depth_m: 0.4,
            height_m: 0.7,
            rgb: [10, 20, 30],
        });
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.custom_items.len(), 1);
        assert_eq!(back.custom_items[0].width_m, 10.0); // clamped
        assert_eq!(back.custom_items[0].catalog_id(), format!("custom-{id}"));
    }

    #[test]
    fn underlay_fit_and_calibration() {
        let room = Room::default(); // 5 x 4
        let mut u = Underlay::fit(&room, 1000, 800, "AAAA".into());
        // Fit: image corners land on room corners.
        assert_eq!(u.room_from_px(0.0, 0.0), (0.0, 4.0));
        let (x, y) = u.room_from_px(1000.0, 800.0);
        assert!((x - 5.0).abs() < 1e-9 && y.abs() < 1e-9);
        // Calibrate: two marks 500 px apart are really 3 m; the first
        // mark's room position must not move.
        let before = u.room_from_px(100.0, 100.0);
        u.calibrate((100.0, 100.0), (600.0, 100.0), 3.0);
        assert!((u.m_per_px - 0.006).abs() < 1e-12);
        let after = u.room_from_px(100.0, 100.0);
        assert!((after.0 - before.0).abs() < 1e-9 && (after.1 - before.1).abs() < 1e-9);
        let (bx, by) = u.room_from_px(600.0, 100.0);
        let dist = ((bx - after.0).powi(2) + (by - after.1).powi(2)).sqrt();
        assert!((dist - 3.0).abs() < 1e-9);
        // Inverse transform round-trips.
        let (rx, ry) = u.room_from_px(250.0, 300.0);
        let (px, py) = u.px_from_room(rx, ry);
        assert!((px - 250.0).abs() < 1e-6 && (py - 300.0).abs() < 1e-6);
    }

    #[test]
    fn annotations_and_thickness_survive_roundtrip_and_legacy_json() {
        let mut p = Project::default();
        let id = p.alloc_id();
        p.annotations.push(Annotation {
            id,
            x_m: 1.0,
            y_m: 2.0,
            text: "TV wall".into(),
        });
        p.room.wall_thickness_m = 0.2;
        let back = Project::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back.annotations.len(), 1);
        assert_eq!(back.room.wall_thickness_m, 0.2);
        // A v0.1 file (no annotations / wall_thickness keys) still loads.
        let legacy = r#"{"format_version":1,"room":{"name":"R","width_m":5.0,
            "depth_m":4.0,"height_m":2.5,"wall_rgb":[1,2,3],"floor_rgb":[4,5,6]},
            "openings":[],"services":[],"items":[],"next_id":1}"#;
        let old = Project::from_json(legacy).unwrap();
        assert!(old.annotations.is_empty());
        assert_eq!(old.room.wall_thickness_m, 0.1);
    }
}
