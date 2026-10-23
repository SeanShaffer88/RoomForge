//! 3D projection maths for the room view. Pure functions: the GUI feeds
//! in camera angles and paints the projected polygons it gets back.
//! Camera: orbit around the room centre with yaw (around the vertical
//! axis) and pitch (down from horizontal), plus a simple perspective
//! divide. Units are metres throughout.

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

/// Shade an [r,g,b] colour by a brightness factor.
pub fn shade(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| ((c as f64 * factor).clamp(0.0, 255.0)) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PlacedItem;

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
