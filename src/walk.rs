//! First-person walkthrough: a true-perspective camera that stands
//! inside the room. All the maths lives here so it is testable
//! off-Windows: camera transform, near-plane clipping, the face
//! list (painter's order, floor pattern included), and collision
//! movement that slides along walls and furniture.

use crate::geometry::{point_in_polygon, rect_cells};
use crate::model::{OpeningKind, Project};
use crate::view3d::{box_corners, box_faces, shade_scene, Scene};

/// Where the visitor stands, and where they are looking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walker {
    /// Position in room coordinates (metres).
    pub pos: (f64, f64),
    /// Compass-style heading: 0 looks north (+y), 90 east (+x).
    pub yaw_deg: f64,
    /// Look up/down, degrees, clamped to ±60.
    pub pitch_deg: f64,
    /// Eye height in metres.
    pub eye_m: f64,
}

impl Default for Walker {
    fn default() -> Self {
        Walker {
            pos: (0.0, 0.0),
            yaw_deg: 0.0,
            pitch_deg: 0.0,
            eye_m: 1.6,
        }
    }
}

impl Walker {
    /// A good starting spot: just inside the south wall, centred,
    /// looking north across the room. Falls back to the room centre
    /// when the doorway spot is blocked or outside an L-shape.
    pub fn start(p: &Project) -> Walker {
        let room = &p.room;
        let outline = room.outline_or_rect();
        let mut w = Walker {
            pos: (room.width_m / 2.0, 0.7),
            ..Walker::default()
        };
        if !point_in_polygon(w.pos.0, w.pos.1, &outline) || w.blocked(p, w.pos) {
            w.pos = (room.width_m / 2.0, room.depth_m / 2.0);
        }
        w
    }

    /// Is a body of radius `BODY_R` at this point inside furniture?
    fn blocked(&self, p: &Project, at: (f64, f64)) -> bool {
        p.items.iter().any(|it| {
            // Inflate the item's footprint by the body radius and
            // test the corners-formed box.
            let (hw, hd) = (it.width_m / 2.0 + BODY_R, it.depth_m / 2.0 + BODY_R);
            let rot = it.rotation_deg.to_radians();
            let (c, s) = (rot.cos(), rot.sin());
            let dx = at.0 - it.x_m;
            let dy = at.1 - it.y_m;
            let lx = dx * c + dy * s;
            let ly = -dx * s + dy * c;
            // Rugs and other flat items can be walked over.
            it.height_m > 0.05 && lx.abs() <= hw && ly.abs() <= hd
        })
    }

    /// Move by (dx, dy) in room space, axis by axis so the visitor
    /// slides along whatever blocks them. Returns the new position.
    pub fn moved(&self, p: &Project, dx: f64, dy: f64) -> (f64, f64) {
        let outline = p.room.outline_or_rect();
        let try_at = |at: (f64, f64)| -> Option<(f64, f64)> {
            // Keep the body inside the room: test the centre and the
            // four radius points against the outline.
            let probes = [
                at,
                (at.0 + BODY_R, at.1),
                (at.0 - BODY_R, at.1),
                (at.0, at.1 + BODY_R),
                (at.0, at.1 - BODY_R),
            ];
            if probes.iter().all(|q| point_in_polygon(q.0, q.1, &outline)) && !self.blocked(p, at) {
                Some(at)
            } else {
                None
            }
        };
        let mut cur = self.pos;
        if let Some(n) = try_at((cur.0 + dx, cur.1)) {
            cur = n;
        }
        if let Some(n) = try_at((cur.0, cur.1 + dy)) {
            cur = n;
        }
        cur
    }

    /// Forward unit vector for the current heading.
    pub fn forward(&self) -> (f64, f64) {
        let y = self.yaw_deg.to_radians();
        (y.sin(), y.cos())
    }

    /// Right unit vector for the current heading.
    pub fn right(&self) -> (f64, f64) {
        let y = self.yaw_deg.to_radians();
        (y.cos(), -y.sin())
    }
}

/// How wide the visitor's body is, for collision purposes.
pub const BODY_R: f64 = 0.22;
/// Nothing closer than this is drawn (the near plane).
pub const NEAR: f64 = 0.08;
/// Vertical field of view, degrees.
pub const FOV_V_DEG: f64 = 68.0;

/// Transform a room point into camera space: x right, y up,
/// z forward (positive in front of the visitor).
pub fn to_camera(w: &Walker, pt: (f64, f64, f64)) -> (f64, f64, f64) {
    let rel = (pt.0 - w.pos.0, pt.1 - w.pos.1, pt.2 - w.eye_m);
    let f = w.forward();
    let r = w.right();
    let cx = rel.0 * r.0 + rel.1 * r.1;
    let cz = rel.0 * f.0 + rel.1 * f.1;
    // Pitch rotates around the right axis.
    let p = w.pitch_deg.clamp(-60.0, 60.0).to_radians();
    let cy = rel.2 * p.cos() - cz * p.sin();
    let cz2 = rel.2 * p.sin() + cz * p.cos();
    (cx, cy, cz2)
}

/// Clip a camera-space polygon against the near plane
/// (Sutherland–Hodgman, one plane). Output vertices all have
/// z >= NEAR; an empty result means the polygon is behind the eye.
pub fn clip_near(poly: &[(f64, f64, f64)]) -> Vec<(f64, f64, f64)> {
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    if poly.is_empty() {
        return out;
    }
    let mut prev = *poly.last().unwrap();
    let mut prev_in = prev.2 >= NEAR;
    for &cur in poly {
        let cur_in = cur.2 >= NEAR;
        if cur_in != prev_in {
            let t = (NEAR - prev.2) / (cur.2 - prev.2);
            out.push((
                prev.0 + (cur.0 - prev.0) * t,
                prev.1 + (cur.1 - prev.1) * t,
                NEAR,
            ));
        }
        if cur_in {
            out.push(cur);
        }
        prev = cur;
        prev_in = cur_in;
    }
    out
}

/// Project a camera-space point to screen space. `focal` is the
/// focal length in pixels; `origin` is the screen centre.
pub fn project_fp(cam_pt: (f64, f64, f64), focal: f64, origin: (f64, f64)) -> (f64, f64) {
    (
        origin.0 + cam_pt.0 * focal / cam_pt.2,
        origin.1 - cam_pt.1 * focal / cam_pt.2,
    )
}

/// One thing to paint in the walk view, in screen space.
pub enum WalkCmd {
    Fill {
        pts: Vec<(f64, f64)>,
        color: [u8; 3],
    },
    Line {
        a: (f64, f64),
        b: (f64, f64),
        color: [u8; 3],
    },
}

/// A paint command with its depth (larger = further away).
pub struct WalkLayer {
    pub depth: f64,
    pub cmd: WalkCmd,
}

/// Build the walk view's paint list for a viewport of `w` x `h`
/// pixels, sorted far to near. Mirrors the orbit renderer's scene:
/// floor with pattern, walls by facing, furniture boxes, window
/// glass last among the fills.
pub fn walk_layers(
    p: &Project,
    w: &Walker,
    scene: Scene,
    width: f64,
    height: f64,
) -> Vec<WalkLayer> {
    let room = &p.room;
    let focal = (height / 2.0) / (FOV_V_DEG / 2.0).to_radians().tan();
    let origin = (width / 2.0, height / 2.0);
    let mut layers: Vec<WalkLayer> = Vec::new();

    let push_poly = |pts3: &[(f64, f64, f64)], color: [u8; 3], layers: &mut Vec<WalkLayer>| {
        let cam: Vec<(f64, f64, f64)> = pts3.iter().map(|&pt| to_camera(w, pt)).collect();
        let clipped = clip_near(&cam);
        if clipped.len() < 3 {
            return;
        }
        let depth = clipped.iter().map(|c| c.2).sum::<f64>() / clipped.len() as f64;
        let pts = clipped
            .iter()
            .map(|&c| project_fp(c, focal, origin))
            .collect();
        layers.push(WalkLayer {
            depth,
            cmd: WalkCmd::Fill { pts, color },
        });
    };

    let outline = room.outline_or_rect();
    let cells = rect_cells(&outline);
    for &(x1, y1, x2, y2) in &cells {
        push_poly(
            &[(x1, y1, 0.0), (x2, y1, 0.0), (x2, y2, 0.0), (x1, y2, 0.0)],
            shade_scene(room.floor_rgb, 0.9, scene),
            &mut layers,
        );
    }
    // Floor pattern lines on the floor, under everything else.
    let line_color = shade_scene(room.floor_rgb, 0.7, scene);
    for (a, b) in room.floor_material.pattern_segments(&cells) {
        let ca = to_camera(w, (a.0, a.1, 0.0));
        let cb = to_camera(w, (b.0, b.1, 0.0));
        let seg = clip_near(&[ca, cb]);
        if seg.len() == 2 {
            layers.push(WalkLayer {
                depth: (seg[0].2 + seg[1].2) / 2.0 - 0.02,
                cmd: WalkCmd::Line {
                    a: project_fp(seg[0], focal, origin),
                    b: project_fp(seg[1], focal, origin),
                    color: line_color,
                },
            });
        }
    }
    // Ceiling, so looking up isn't a void.
    push_poly(
        &[
            (0.0, 0.0, room.height_m),
            (room.width_m, 0.0, room.height_m),
            (room.width_m, room.depth_m, room.height_m),
            (0.0, room.depth_m, room.height_m),
        ],
        shade_scene(room.wall_rgb, 0.55, scene),
        &mut layers,
    );
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (nx, ny) = (dy, -dx);
        let bright = if ny.abs() > nx.abs() {
            if ny > 0.0 {
                0.95
            } else {
                0.75
            }
        } else if nx > 0.0 {
            0.65
        } else {
            0.85
        };
        push_poly(
            &[
                (a.0, a.1, 0.0),
                (b.0, b.1, 0.0),
                (b.0, b.1, room.height_m),
                (a.0, a.1, room.height_m),
            ],
            shade_scene(room.wall_rgb, bright, scene),
            &mut layers,
        );
    }
    for it in &p.items {
        let corners = box_corners(it);
        for (idx, bright) in box_faces() {
            let quad = [
                corners[idx[0]],
                corners[idx[1]],
                corners[idx[2]],
                corners[idx[3]],
            ];
            push_poly(&quad, shade_scene(it.rgb, bright, scene), &mut layers);
        }
    }
    // Window glass.
    for o in &p.openings {
        if o.kind != OpeningKind::Window {
            continue;
        }
        let quad = crate::view3d::wall_opening_quad(p, o);
        push_poly(&quad, scene.glass(), &mut layers);
    }

    layers.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahead_is_centre_and_behind_is_negative() {
        let w = Walker {
            pos: (2.5, 0.7),
            ..Walker::default()
        };
        let ahead = to_camera(&w, (2.5, 3.7, 1.6));
        assert!(ahead.0.abs() < 1e-9 && ahead.1.abs() < 1e-9 && (ahead.2 - 3.0).abs() < 1e-9);
        let behind = to_camera(&w, (2.5, 0.0, 1.6));
        assert!(behind.2 < 0.0);
        let right = to_camera(&w, (3.5, 0.7, 1.6));
        assert!(
            right.0 > 0.9 && right.2.abs() < 1e-9,
            "east is right when facing north: {right:?}"
        );
        let (sx, sy) = project_fp(ahead, 500.0, (400.0, 300.0));
        assert!((sx - 400.0).abs() < 1e-9 && (sy - 300.0).abs() < 1e-9);
    }

    #[test]
    fn near_clip_keeps_the_front_part() {
        // A wall quad straddling the eye: two verts behind, two ahead.
        let quad = vec![
            (-1.0, -1.0, -1.0),
            (1.0, -1.0, -1.0),
            (1.0, 1.0, 2.0),
            (-1.0, 1.0, 2.0),
        ];
        let clipped = clip_near(&quad);
        assert!(clipped.len() >= 3, "clipped to {} verts", clipped.len());
        assert!(clipped.iter().all(|c| c.2 >= NEAR - 1e-12));
        let behind = vec![(0.0, 0.0, -1.0), (1.0, 0.0, -2.0), (0.0, 1.0, -1.0)];
        assert!(clip_near(&behind).is_empty());
    }

    #[test]
    fn walker_stops_at_walls_and_slides_along_furniture() {
        let p = Project::default(); // empty 5x4 room
        let mut w = Walker::start(&p);
        assert!((w.pos.1 - 0.7).abs() < 1e-9);
        // Walking north for a long way stops BODY_R short of the wall.
        for _ in 0..100 {
            w.pos = w.moved(&p, 0.0, 0.1);
        }
        assert!(
            w.pos.1 <= 4.0 - BODY_R + 1e-9 && w.pos.1 > 3.5,
            "stopped at {}",
            w.pos.1
        );
        // Slide: pushing diagonally into the west wall still moves north.
        let w2 = Walker {
            pos: (0.3, 2.0),
            ..Walker::default()
        };
        let n = w2.moved(&p, -0.2, 0.2);
        assert!(n.0 >= BODY_R - 1e-9 && n.1 > 2.0, "slid to {n:?}");
    }

    #[test]
    fn walk_layers_cover_the_room() {
        let p = Project::default();
        let w = Walker::start(&p);
        let layers = walk_layers(&p, &w, Scene::Day, 900.0, 600.0);
        assert!(layers.len() >= 5, "floor + ceiling + walls at least");
        // Sorted far to near.
        assert!(layers.windows(2).all(|pair| pair[0].depth >= pair[1].depth));
        // Night differs from day (scene flows through).
        let night = walk_layers(&p, &w, Scene::Night, 900.0, 600.0);
        let day_col = match &layers[0].cmd {
            WalkCmd::Fill { color, .. } => *color,
            _ => panic!("first layer should be a fill"),
        };
        let night_col = match &night[0].cmd {
            WalkCmd::Fill { color, .. } => *color,
            _ => panic!("first layer should be a fill"),
        };
        assert_ne!(day_col, night_col);
    }
}
