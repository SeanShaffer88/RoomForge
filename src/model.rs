//! RoomForge project model: a room to scale, its openings and services,
//! and the furniture placed in it. All lengths are metres unless a field
//! name says otherwise. This module is pure data + rules so it can be
//! tested off-Windows with no GUI involved.

use serde::{Deserialize, Serialize};

/// Current project-file format version. Readers accept this and older
/// versions they know how to migrate (see `Project::from_json`).
pub const FORMAT_VERSION: u32 = 1;

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
        let len = self.wall.length(room);
        self.width_m = self.width_m.clamp(0.3, len.max(0.3));
        self.offset_m = self.offset_m.clamp(0.0, (len - self.width_m).max(0.0));
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
        let len = self.wall.length(room);
        let half = self.kind.width_m() / 2.0;
        self.offset_m = self.offset_m.clamp(half, (len - half).max(half));
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

/// The room shell. v1 rooms are rectangular: four walls described by
/// width (W–E) and depth (S–N) plus a ceiling height.
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
    /// Wall colour as [r, g, b] 0–255.
    pub wall_rgb: [u8; 3],
    /// Floor colour as [r, g, b] 0–255.
    pub floor_rgb: [u8; 3],
}

impl Room {
    pub fn area_m2(&self) -> f64 {
        self.width_m * self.depth_m
    }
    pub fn clamp_dims(&mut self) {
        self.width_m = self.width_m.clamp(1.0, 30.0);
        self.depth_m = self.depth_m.clamp(1.0, 30.0);
        self.height_m = self.height_m.clamp(1.8, 6.0);
        self.wall_thickness_m = self.wall_thickness_m.clamp(0.05, 0.6);
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
        self.x_m = self.x_m.clamp(0.0, room.width_m);
        self.y_m = self.y_m.clamp(0.0, room.depth_m);
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
    /// Keep the item's centre inside the room (with a small margin so a
    /// rotated item can still be grabbed at the edges).
    pub fn clamp_inside(&mut self, room: &Room) {
        self.x_m = self.x_m.clamp(0.0, room.width_m);
        self.y_m = self.y_m.clamp(0.0, room.depth_m);
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
        let max_id = p
            .items
            .iter()
            .map(|i| i.id)
            .chain(p.openings.iter().map(|o| o.id))
            .chain(p.services.iter().map(|s| s.id))
            .chain(p.annotations.iter().map(|a| a.id))
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

    /// Human-readable warnings: furniture overlaps and items covering a
    /// radiator or pipe. Doors swinging into furniture is also reported.
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        for i in 0..self.items.len() {
            for j in (i + 1)..self.items.len() {
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
