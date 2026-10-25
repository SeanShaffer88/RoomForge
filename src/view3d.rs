//! 3D projection maths for the room view. Pure functions: the GUI feeds
//! in camera angles and paints the projected polygons it gets back.
//! Camera: orbit around the room centre with yaw (around the vertical
//! axis) and pitch (down from horizontal), plus a simple perspective
//! divide. Units are metres throughout.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Orbit angle around the room, degrees. 0 = looking from the south.
    pub yaw_deg: f64,
    /// Tilt down from horizontal, degrees (10–80 sensible).
    pub pitch_deg: f64,
    /// Pixels per metre at the room centre.
    pub scale: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            yaw_deg: 35.0,
            pitch_deg: 50.0,
            scale: 70.0,
        }
    }
}

/// Project a room-space point (x east, y north, z up, metres) to screen
/// space relative to `origin` (the screen point for the room centre at
/// floor level). Returns (sx, sy, depth) — larger depth is farther away,
/// for painter's-algorithm sorting.
pub fn project(
    p: (f64, f64, f64),
    centre: (f64, f64),
    cam: &Camera,
    origin: (f64, f64),
) -> (f64, f64, f64) {
    let (dx, dy) = (p.0 - centre.0, p.1 - centre.1);
    let yaw = cam.yaw_deg.to_radians();
    let pitch = cam.pitch_deg.to_radians();
    // Yaw around the vertical axis.
    let (rx, ry) = (
        dx * yaw.cos() - dy * yaw.sin(),
        dx * yaw.sin() + dy * yaw.cos(),
    );
    // Pitch: y tilts away, z lifts.
    let (ry2, rz2) = (
        ry * pitch.cos() - p.2 * pitch.sin(),
        ry * pitch.sin() + p.2 * pitch.cos(),
    );
    // Mild perspective: things farther away shrink a little.
    let persp = 1.0 / (1.0 + ry2.max(-8.0) * 0.03);
    (
        origin.0 + rx * cam.scale * persp,
        origin.1 - rz2 * cam.scale * persp,
        ry2,
    )
}

/// The 3D quad of a wall opening, slightly proud of the wall surface.
pub fn wall_opening_quad(
    p: &crate::model::Project,
    o: &crate::model::Opening,
) -> [(f64, f64, f64); 4] {
    use crate::model::Wall;
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

/// The eight 3D corners of a placed item (a box), in room space.
pub fn box_corners(it: &crate::model::PlacedItem) -> [(f64, f64, f64); 8] {
    let plan = it.corners();
    let mut out = [(0.0, 0.0, 0.0); 8];
    for (i, &(x, y)) in plan.iter().enumerate() {
        out[i] = (x, y, 0.0);
        out[i + 4] = (x, y, it.height_m);
    }
    out
}

/// Box faces as (corner indices, brightness 0.55–1.0). The brightness is
/// a fixed light-from-above/south shading so the painter needs no
/// lighting code of its own.
/// World-space triangles for a placed item that carries an
/// imported 3D model: the prepared unit meshes scaled onto the
/// item's footprint and height, rotated and translated with it.
/// Brightness comes from each triangle's world normal lit by the
/// same fixed light the box shades imply, double-sided (imported
/// models often have inconsistent winding). Returns
/// (points, brightness, base colour) per triangle.
/// One rendered model triangle: world points, brightness, colour.
pub type MeshTri = ([(f64, f64, f64); 3], f64, [u8; 3]);

pub fn mesh_tris(
    it: &crate::model::PlacedItem,
    prep: &crate::model3d::PreparedModel,
) -> Vec<MeshTri> {
    let rad = it.rotation_deg.to_radians();
    let (s, c) = rad.sin_cos();
    let sx = it.width_m;
    let sy = if prep.unit_depth > 1e-9 {
        it.depth_m / prep.unit_depth
    } else {
        it.width_m
    };
    let sz = if prep.unit_height > 1e-9 {
        it.height_m / prep.unit_height
    } else {
        it.width_m
    };
    let mut out = Vec::new();
    for mesh in &prep.meshes {
        for tri in &mesh.tris {
            let w = tri.map(|p| {
                let lx = p[0] as f64 * sx;
                let ly = p[1] as f64 * sy;
                (
                    it.x_m + lx * c - ly * s,
                    it.y_m + lx * s + ly * c,
                    p[2] as f64 * sz,
                )
            });
            let bright = face_bright(w[0], w[1], w[2]);
            out.push((w, bright, mesh.rgb));
        }
    }
    out
}

/// Brightness of a face from its normal, under the renderers'
/// fixed light (roughly from the south-west above). Double-sided:
/// a face lit from behind still reads.
pub fn face_bright(a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)) -> f64 {
    let l = (-0.35, -0.5, 0.79);
    let u = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
    let v = (c.0 - a.0, c.1 - a.1, c.2 - a.2);
    let n = (
        u.1 * v.2 - u.2 * v.1,
        u.2 * v.0 - u.0 * v.2,
        u.0 * v.1 - u.1 * v.0,
    );
    let len = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt().max(1e-12);
    let d = ((n.0 / len) * l.0 + (n.1 / len) * l.1 + (n.2 / len) * l.2).abs();
    (0.5f64 + 0.55 * d).clamp(0.3, 1.0)
}

/// How a furniture front panel moves when it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontKind {
    /// Door hinged on its left edge (swings out).
    DoorLeft,
    /// Door hinged on its right edge.
    DoorRight,
    /// Door hinged on its bottom edge (oven, dishwasher — drops down).
    DoorBottom,
    /// Drawer front (slides straight out).
    Drawer,
}

/// One front panel of a furniture piece, in item space: `u` runs
/// 0..1 across the width, `z` is metres from the floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontPanel {
    pub u0: f64,
    pub u1: f64,
    pub z0: f64,
    pub z1: f64,
    pub kind: FrontKind,
}

/// The front panels a catalog piece has (wardrobe doors, chest
/// drawers, an oven's drop-down door…). Keyed by catalog id;
/// anything else — sofas, tables, shelves, custom and imported
/// pieces — has none. The front of a piece faces local +y (south
/// when unrotated), the same side the 2D glyphs draw as the front.
pub fn front_panels(it: &crate::model::PlacedItem) -> Vec<FrontPanel> {
    let h = it.height_m;
    let door = |u0: f64, u1: f64, kind: FrontKind| FrontPanel {
        u0,
        u1,
        z0: 0.02,
        z1: (h - 0.02).max(0.05),
        kind,
    };
    match it.catalog_id.as_str() {
        "wardrobe-2" | "sideboard" | "tv-bench" | "shoe-cabinet" | "sink-unit" => vec![
            door(0.01, 0.5, FrontKind::DoorLeft),
            door(0.5, 0.99, FrontKind::DoorRight),
        ],
        "wardrobe-3" => vec![
            door(0.01, 0.34, FrontKind::DoorLeft),
            door(0.34, 0.67, FrontKind::DoorLeft),
            door(0.67, 0.99, FrontKind::DoorRight),
        ],
        "base-cabinet-60" => vec![door(0.01, 0.99, FrontKind::DoorLeft)],
        "fridge" => vec![door(0.01, 0.99, FrontKind::DoorRight)],
        "dishwasher" => vec![door(0.01, 0.99, FrontKind::DoorBottom)],
        "cooker" => vec![FrontPanel {
            u0: 0.01,
            u1: 0.99,
            z0: 0.05,
            z1: h * 0.8,
            kind: FrontKind::DoorBottom,
        }],
        "chest-4" => (0..4)
            .map(|i| {
                let band = h / 4.0;
                FrontPanel {
                    u0: 0.03,
                    u1: 0.97,
                    z0: i as f64 * band + 0.012,
                    z1: (i + 1) as f64 * band - 0.012,
                    kind: FrontKind::Drawer,
                }
            })
            .collect(),
        "bedside-table" => vec![FrontPanel {
            u0: 0.05,
            u1: 0.95,
            z0: h * 0.45,
            z1: (h - 0.03).max(h * 0.5),
            kind: FrontKind::Drawer,
        }],
        _ => Vec::new(),
    }
}

/// A furniture face quad with its brightness and base colour,
/// ready for any of the renderers.
pub type FaceQuad = ([(f64, f64, f64); 4], f64, [u8; 3]);

/// The world-space quads of an item's front panels, with `open`
/// from 0 (closed) to 1 (fully open). Closed panels sit a hair
/// proud of the front face, so pieces read as having doors and
/// drawers even when shut.
pub fn front_quads(it: &crate::model::PlacedItem, open: f64) -> Vec<FaceQuad> {
    let open = open.clamp(0.0, 1.0);
    let rad = it.rotation_deg.to_radians();
    let (s, c) = rad.sin_cos();
    let w = it.width_m;
    let front_y = it.depth_m / 2.0 + 0.012;
    let rgb = [
        (it.rgb[0] as f64 * 0.85) as u8,
        (it.rgb[1] as f64 * 0.85) as u8,
        (it.rgb[2] as f64 * 0.85) as u8,
    ];
    let mut out = Vec::new();
    for p in front_panels(it) {
        // Panel corners in local (lx, ly, z), before the opening
        // motion: (u0,z0), (u1,z0), (u1,z1), (u0,z1) on the front plane.
        let lx = |u: f64| (u - 0.5) * w;
        let corners = [
            (lx(p.u0), front_y, p.z0),
            (lx(p.u1), front_y, p.z0),
            (lx(p.u1), front_y, p.z1),
            (lx(p.u0), front_y, p.z1),
        ];
        let moved: Vec<(f64, f64, f64)> = corners
            .iter()
            .map(|&(x, y, z)| match p.kind {
                FrontKind::Drawer => (x, y + open * (it.depth_m * 0.7).min(0.32), z),
                FrontKind::DoorLeft | FrontKind::DoorRight => {
                    // Swing around the hinge's vertical line.
                    let hx = if p.kind == FrontKind::DoorLeft {
                        lx(p.u0)
                    } else {
                        lx(p.u1)
                    };
                    let a = if p.kind == FrontKind::DoorLeft {
                        open * 100.0f64.to_radians()
                    } else {
                        -open * 100.0f64.to_radians()
                    };
                    let (rx, ry) = (x - hx, y - front_y);
                    (
                        hx + rx * a.cos() - ry * a.sin(),
                        front_y + rx * a.sin() + ry * a.cos(),
                        z,
                    )
                }
                FrontKind::DoorBottom => {
                    // Drop around the bottom edge's horizontal line,
                    // which sits on the front plane at z0.
                    let b = open * 75.0f64.to_radians();
                    let (dy, rz) = (y - front_y, z - p.z0);
                    (
                        x,
                        front_y + dy * b.cos() + rz * b.sin(),
                        p.z0 - dy * b.sin() + rz * b.cos(),
                    )
                }
            })
            .collect();
        let world = [
            (
                it.x_m + moved[0].0 * c - moved[0].1 * s,
                it.y_m + moved[0].0 * s + moved[0].1 * c,
                moved[0].2,
            ),
            (
                it.x_m + moved[1].0 * c - moved[1].1 * s,
                it.y_m + moved[1].0 * s + moved[1].1 * c,
                moved[1].2,
            ),
            (
                it.x_m + moved[2].0 * c - moved[2].1 * s,
                it.y_m + moved[2].0 * s + moved[2].1 * c,
                moved[2].2,
            ),
            (
                it.x_m + moved[3].0 * c - moved[3].1 * s,
                it.y_m + moved[3].0 * s + moved[3].1 * c,
                moved[3].2,
            ),
        ];
        let bright = face_bright(world[0], world[1], world[2]);
        out.push((world, bright, rgb));
    }
    out
}

/// A copy of the item shifted away from the room centre for the
/// exploded view (`amount` 0..1). Pieces near the centre barely
/// move; the shift is capped so nothing leaves the room by much.
pub fn exploded(
    it: &crate::model::PlacedItem,
    centre: (f64, f64),
    amount: f64,
) -> crate::model::PlacedItem {
    let mut c = it.clone();
    if amount <= 0.0 {
        return c;
    }
    let (dx, dy) = (it.x_m - centre.0, it.y_m - centre.1);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 0.05 {
        return c;
    }
    let shift = (amount * 0.55 * dist).min(1.1);
    c.x_m += dx / dist * shift;
    c.y_m += dy / dist * shift;
    c
}

pub fn box_faces() -> [([usize; 4], f64); 6] {
    [
        ([0, 1, 2, 3], 0.62), // base (rarely visible)
        ([4, 5, 6, 7], 1.0),  // top
        ([0, 1, 5, 4], 0.78), // south
        ([1, 2, 6, 5], 0.7),  // east
        ([2, 3, 7, 6], 0.86), // north
        ([3, 0, 4, 7], 0.94), // west
    ]
}

/// The skirting-board quad for one outline edge: a 9 cm strip
/// standing just inside the wall line (inset 1.2 cm so it reads as
/// a separate board against the wall).
pub fn skirting_quad(a: (f64, f64), b: (f64, f64)) -> [(f64, f64, f64); 4] {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt().max(1e-9);
    // Inward (left of travel for our CCW outlines), 1.2 cm in.
    let (nx, ny) = (-dy / len * 0.012, dx / len * 0.012);
    let (ax, ay) = (a.0 + nx, a.1 + ny);
    let (bx, by) = (b.0 + nx, b.1 + ny);
    [(ax, ay, 0.0), (bx, by, 0.0), (bx, by, 0.09), (ax, ay, 0.09)]
}

/// A soft-shadow footprint for an item: its footprint grown by
/// 5 cm on every side, lying on the floor. Flat items (rugs) get
/// no shadow — they ARE the floor decoration.
pub fn shadow_quad(it: &crate::model::PlacedItem) -> Option<[(f64, f64, f64); 4]> {
    if it.height_m <= 0.05 {
        return None;
    }
    let rot = it.rotation_deg.to_radians();
    let (c, s) = (rot.cos(), rot.sin());
    let hw = it.width_m / 2.0 + 0.05;
    let hd = it.depth_m / 2.0 + 0.05;
    let corner = |lx: f64, ly: f64| (it.x_m + lx * c - ly * s, it.y_m + lx * s + ly * c, 0.0);
    Some([
        corner(-hw, -hd),
        corner(hw, -hd),
        corner(hw, hd),
        corner(-hw, hd),
    ])
}

/// Shade an [r,g,b] colour by a brightness factor.
pub fn shade(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| ((c as f64 * factor).clamp(0.0, 255.0)) as u8)
}

/// A lighting mood for the 3D view and snapshots. Scenes are global
/// ambience — an overall brightness, a colour tint, a backdrop and a
/// window-glass colour — not per-lamp lighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Scene {
    #[default]
    Day,
    Evening,
    Night,
}

impl Scene {
    pub fn label(self) -> &'static str {
        match self {
            Scene::Day => "Day",
            Scene::Evening => "Evening",
            Scene::Night => "Night",
        }
    }
    /// Overall brightness multiplier.
    pub fn ambient(self) -> f64 {
        match self {
            Scene::Day => 1.0,
            Scene::Evening => 0.8,
            Scene::Night => 0.48,
        }
    }
    /// Per-channel colour tint (warm at evening, cool at night).
    pub fn tint(self) -> (f64, f64, f64) {
        match self {
            Scene::Day => (1.0, 1.0, 1.0),
            Scene::Evening => (1.06, 0.93, 0.78),
            Scene::Night => (0.8, 0.9, 1.12),
        }
    }
    /// Backdrop colour behind the room.
    pub fn background(self) -> [u8; 3] {
        match self {
            Scene::Day => [8, 8, 8],
            Scene::Evening => [14, 9, 7],
            Scene::Night => [3, 5, 12],
        }
    }
    /// Window glass colour: sky by day, sunset in the evening,
    /// moonlight at night.
    pub fn glass(self) -> [u8; 3] {
        match self {
            Scene::Day => [120, 190, 240],
            Scene::Evening => [255, 176, 102],
            Scene::Night => [130, 165, 235],
        }
    }
    /// Glass opacity for the live view (the snapshot paints it solid).
    pub fn glass_alpha(self) -> u8 {
        match self {
            Scene::Day => 200,
            Scene::Evening => 185,
            Scene::Night => 140,
        }
    }
}

/// How the 3D views present surfaces: true colours, or a
/// monochrome "clay" look (every surface a warm grey by its
/// brightness) for judging shapes and layout without colour noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Style {
    #[default]
    Coloured,
    Clay,
    /// Technical-drawing look: pale paper surfaces (edges are
    /// drawn dark by the renderers).
    Outline,
}

/// Map a surface colour through a render style.
pub fn style_rgb(rgb: [u8; 3], style: Style) -> [u8; 3] {
    match style {
        Style::Coloured => rgb,
        Style::Clay => {
            let l = 0.299 * rgb[0] as f64 + 0.587 * rgb[1] as f64 + 0.114 * rgb[2] as f64;
            [
                (l * 1.05).clamp(0.0, 255.0) as u8,
                l.clamp(0.0, 255.0) as u8,
                (l * 0.9).clamp(0.0, 255.0) as u8,
            ]
        }
        Style::Outline => {
            let l = 0.299 * rgb[0] as f64 + 0.587 * rgb[1] as f64 + 0.114 * rgb[2] as f64;
            let p = 206.0 + l * 0.17;
            [
                (p * 1.02).clamp(0.0, 250.0) as u8,
                p.clamp(0.0, 250.0) as u8,
                (p * 0.97).clamp(0.0, 250.0) as u8,
            ]
        }
    }
}

/// How much wall the 3D views draw: the full height, a half-height
/// cut or a low cut — a simple section view for looking into the
/// room. Rendering only; the plan itself is untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WallCut {
    #[default]
    Full,
    Half,
    Low,
}

impl WallCut {
    /// The wall-top height to render for a room of `room_h` metres.
    pub fn cut_height(self, room_h: f64) -> f64 {
        match self {
            WallCut::Full => room_h,
            WallCut::Half => room_h * 0.55,
            WallCut::Low => room_h.min(1.05),
        }
    }
}

/// Shade a colour by a face factor and then by the scene's light.
pub fn shade_scene(rgb: [u8; 3], factor: f64, scene: Scene) -> [u8; 3] {
    let (tr, tg, tb) = scene.tint();
    let tints = [tr, tg, tb];
    let f = factor * scene.ambient();
    std::array::from_fn(|i| ((rgb[i] as f64 * f * tints[i]).clamp(0.0, 255.0)) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PlacedItem;

    #[test]
    fn clay_style_flattens_to_warm_grey() {
        assert_eq!(style_rgb([10, 200, 90], Style::Coloured), [10, 200, 90]);
        let clay = style_rgb([255, 0, 0], Style::Clay);
        // Warm grey: channels close together, red slightly ahead.
        assert!(clay[0] >= clay[1] && clay[1] >= clay[2], "{clay:?}");
        assert!(clay[0] - clay[2] < 20, "{clay:?}");
        assert!((70..=90).contains(&clay[1]), "{clay:?}");
        // A grey stays the same grey (within the warm tint).
        let g = style_rgb([128, 128, 128], Style::Clay);
        assert!((120..=140).contains(&g[1]), "{g:?}");
    }

    #[test]
    fn wall_cut_heights() {
        assert_eq!(WallCut::Full.cut_height(2.6), 2.6);
        assert!((WallCut::Half.cut_height(2.6) - 1.43).abs() < 1e-9);
        assert_eq!(WallCut::Low.cut_height(2.6), 1.05);
        assert_eq!(WallCut::Low.cut_height(0.9), 0.9);
    }

    fn placed(catalog_id: &str, x: f64, y: f64, h: f64) -> PlacedItem {
        PlacedItem {
            id: 1,
            catalog_id: catalog_id.into(),
            name: "T".into(),
            category: "Storage".into(),
            x_m: x,
            y_m: y,
            width_m: 1.2,
            depth_m: 0.6,
            height_m: h,
            rotation_deg: 0.0,
            rgb: [100, 100, 100],
        }
    }

    #[test]
    fn fronts_by_kind() {
        let w = placed("wardrobe-2", 2.0, 2.0, 2.0);
        let ps = front_panels(&w);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].kind, FrontKind::DoorLeft);
        assert_eq!(ps[1].kind, FrontKind::DoorRight);
        let chest = placed("chest-4", 2.0, 2.0, 1.2);
        let ps = front_panels(&chest);
        assert_eq!(ps.len(), 4);
        assert!(ps.iter().all(|p| p.kind == FrontKind::Drawer));
        assert!(front_panels(&placed("bookcase", 2.0, 2.0, 1.8)).is_empty());
        assert!(front_panels(&placed("custom-9", 2.0, 2.0, 1.0)).is_empty());
    }

    #[test]
    fn closed_fronts_flush_open_ones_move() {
        let w = placed("wardrobe-2", 2.0, 2.0, 2.0);
        let closed = front_quads(&w, 0.0);
        assert_eq!(closed.len(), 2);
        // Closed: every point on the front plane (y = 2 + 0.3 + 0.012).
        for (quad, _, _) in &closed {
            for pt in quad {
                assert!((pt.1 - 2.312).abs() < 1e-9, "{pt:?}");
            }
        }
        // Open: the doors swing out toward +y.
        let open = front_quads(&w, 1.0);
        let max_y = open
            .iter()
            .flat_map(|(q, _, _)| q.iter().map(|p| p.1))
            .fold(f64::MIN, f64::max);
        assert!(max_y > 2.7, "doors swing out (max y {max_y})");
        // Drawers slide straight out by the capped travel.
        let chest = placed("chest-4", 2.0, 2.0, 1.2);
        let c0 = front_quads(&chest, 0.0)[0].0[0].1;
        let c1 = front_quads(&chest, 1.0)[0].0[0].1;
        assert!((c1 - c0 - 0.32).abs() < 1e-9, "drawer travel {c0} -> {c1}");
        // The oven door drops: its top edge ends lower than it started.
        let cooker = placed("cooker", 2.0, 2.0, 0.9);
        let top_closed = front_quads(&cooker, 0.0)[0].0[2].2;
        let top_open = front_quads(&cooker, 1.0)[0].0[2].2;
        assert!(
            top_open < top_closed - 0.2,
            "oven door drops {top_closed} -> {top_open}"
        );
    }

    #[test]
    fn exploded_view_shifts_outward() {
        let it = placed("wardrobe-2", 4.5, 3.5, 2.0);
        let still = exploded(&it, (2.5, 2.0), 0.0);
        assert_eq!((still.x_m, still.y_m), (4.5, 3.5));
        let out = exploded(&it, (2.5, 2.0), 1.0);
        assert!((out.x_m - 5.38).abs() < 1e-9, "{}", out.x_m);
        assert!((out.y_m - 4.16).abs() < 1e-9, "{}", out.y_m);
        let centre = placed("wardrobe-2", 2.5, 2.0, 2.0);
        let c = exploded(&centre, (2.5, 2.0), 1.0);
        assert_eq!((c.x_m, c.y_m), (2.5, 2.0));
    }

    #[test]
    fn outline_style_is_paper() {
        for rgb in [[10, 10, 10], [250, 250, 250], [200, 30, 30]] {
            let o = style_rgb(rgb, Style::Outline);
            assert!(o.iter().all(|&ch| ch >= 195), "{o:?}");
            let spread = *o.iter().max().unwrap() - *o.iter().min().unwrap();
            assert!(spread < 16, "{o:?}");
        }
    }

    #[test]
    fn mesh_tris_fit_the_footprint() {
        let def = crate::model::CustomModelDef {
            id: 1,
            name: "T".into(),
            format: "obj".into(),
            data_base64: String::new(),
            unit_depth: 0.5,
            unit_height: 0.5,
            natural_width_m: 2.0,
        };
        let prep = crate::model3d::PreparedModel {
            meshes: vec![crate::model3d::Mesh {
                tris: vec![[[-0.5, -0.25, 0.0], [0.5, -0.25, 0.0], [0.0, 0.25, 0.5]]],
                rgb: [10, 20, 30],
            }],
            unit_depth: def.unit_depth,
            unit_height: def.unit_height,
            natural_width: 2.0,
        };
        let it = PlacedItem {
            id: 1,
            catalog_id: "model-1".into(),
            name: "T".into(),
            category: "3D models".into(),
            x_m: 2.0,
            y_m: 2.0,
            width_m: 2.0,
            depth_m: 1.0,
            height_m: 1.0,
            rotation_deg: 0.0,
            rgb: [0, 0, 0],
        };
        let tris = mesh_tris(&it, &prep);
        assert_eq!(tris.len(), 1);
        let (pts, bright, rgb) = &tris[0];
        assert_eq!(*rgb, [10, 20, 30]);
        assert!((0.3..=1.0).contains(bright));
        // x spans the item's width around its centre; z tops at height.
        let xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
        assert!((xs.iter().cloned().fold(f64::MIN, f64::max) - 3.0).abs() < 1e-9);
        assert!((pts[2].2 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn skirting_and_shadow_quads() {
        // South edge of a 5x4 room runs (0,0)->(5,0); skirting sits
        // 1.2 cm inside (positive y) and 9 cm tall.
        let q = skirting_quad((0.0, 0.0), (5.0, 0.0));
        assert!((q[0].1 - 0.012).abs() < 1e-9 && (q[2].2 - 0.09).abs() < 1e-9);
        // North edge runs (5,4)->(0,4); inside is negative y.
        let q2 = skirting_quad((5.0, 4.0), (0.0, 4.0));
        assert!((q2[0].1 - (4.0 - 0.012)).abs() < 1e-9);
        let mut it = PlacedItem {
            id: 1,
            catalog_id: "x".into(),
            name: "Sofa".into(),
            category: "Test".into(),
            x_m: 2.0,
            y_m: 2.0,
            width_m: 2.0,
            depth_m: 1.0,
            height_m: 0.85,
            rotation_deg: 0.0,
            rgb: [1, 2, 3],
        };
        let sh = shadow_quad(&it).unwrap();
        assert!((sh[0].0 - (2.0 - 1.05)).abs() < 1e-9 && (sh[2].1 - (2.0 + 0.55)).abs() < 1e-9);
        it.height_m = 0.02;
        assert!(shadow_quad(&it).is_none(), "rugs cast no shadow");
    }

    #[test]
    fn day_scene_is_the_plain_shading() {
        assert_eq!(
            shade_scene([100, 200, 50], 0.5, Scene::Day),
            shade([100, 200, 50], 0.5)
        );
        assert_eq!(
            shade_scene([216, 210, 200], 0.9, Scene::Day),
            shade([216, 210, 200], 0.9)
        );
    }

    #[test]
    fn evening_is_warm_and_night_is_dark_and_cool() {
        let grey = [200, 200, 200];
        let ev = shade_scene(grey, 1.0, Scene::Evening);
        assert!(ev[0] > ev[2], "evening should lean warm: {ev:?}");
        let ni = shade_scene(grey, 1.0, Scene::Night);
        assert!(ni[2] > ni[0], "night should lean cool: {ni:?}");
        assert!(ni[0] < ev[0], "night darker than evening");
        let day = shade_scene(grey, 1.0, Scene::Day);
        assert!(
            day[0] > ev[0] && ev[0] > ni[0],
            "brightness order day > evening > night"
        );
        assert_ne!(Scene::Day.background(), Scene::Night.background());
        assert_ne!(Scene::Day.glass(), Scene::Evening.glass());
    }

    #[test]
    fn centre_projects_to_origin_at_floor_level() {
        let cam = Camera::default();
        let (sx, sy, _) = project((2.5, 2.0, 0.0), (2.5, 2.0), &cam, (400.0, 300.0));
        assert!((sx - 400.0).abs() < 1e-6);
        assert!((sy - 300.0).abs() < 1e-6);
    }

    #[test]
    fn higher_points_project_higher_on_screen() {
        let cam = Camera::default();
        let (_, low, _) = project((2.5, 2.0, 0.0), (2.5, 2.0), &cam, (400.0, 300.0));
        let (_, high, _) = project((2.5, 2.0, 1.5), (2.5, 2.0), &cam, (400.0, 300.0));
        assert!(high < low, "screen y grows downward, so up must be smaller");
    }

    #[test]
    fn box_has_matching_plan_corners() {
        let it = PlacedItem {
            id: 1,
            catalog_id: "x".into(),
            name: "X".into(),
            category: "Other".into(),
            x_m: 1.0,
            y_m: 1.0,
            width_m: 2.0,
            depth_m: 1.0,
            height_m: 0.5,
            rotation_deg: 0.0,
            rgb: [0, 0, 0],
        };
        let corners = box_corners(&it);
        assert_eq!(corners[0], (0.0, 0.5, 0.0));
        assert_eq!(corners[6], (2.0, 1.5, 0.5));
        assert_eq!(box_faces().len(), 6);
        assert_eq!(shade([100, 200, 50], 0.5), [50, 100, 25]);
    }
}
