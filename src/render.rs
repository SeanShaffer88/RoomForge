//! Offline snapshot renderer: paints the 3D view into an image with a
//! small software rasterizer (painter's algorithm, far to near). No
//! GPU, no window — so a plan can be shared as a picture from
//! anywhere, and the renderer is testable off-Windows.

use image::{Rgb, RgbImage};

use crate::geometry::rect_cells;
use crate::model::{OpeningKind, Project};
use crate::view3d::{box_corners, box_faces, project, shade, wall_opening_quad, Camera};

/// Projects a room-space point to (screen point, depth).
type Projector<'a> = dyn Fn((f64, f64, f64)) -> ((f64, f64), f64) + 'a;

enum Cmd {
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

struct Layered {
    depth: f64,
    cmd: Cmd,
}

/// Render the project's 3D view (default orbit camera) to an RGB
/// image of the given pixel size.
pub fn render_snapshot(p: &Project, width: u32, height: u32) -> RgbImage {
    let mut img = RgbImage::from_pixel(width, height, Rgb([8, 8, 8]));
    if width == 0 || height == 0 {
        return img;
    }
    let room = &p.room;
    let centre = (room.width_m / 2.0, room.depth_m / 2.0);
    let fit = (width.min(height) as f64 * 0.92) / (room.width_m.max(room.depth_m) * 1.35);
    let cam = Camera {
        yaw_deg: 35.0,
        pitch_deg: 50.0,
        scale: fit,
    };
    let origin = (width as f64 / 2.0, height as f64 * 0.60);
    let pr = |pt: (f64, f64, f64)| -> ((f64, f64), f64) {
        let (sx, sy, d) = project(pt, centre, &cam, origin);
        ((sx, sy), d)
    };

    let mut layers: Vec<Layered> = Vec::new();
    fn push_quad(
        layers: &mut Vec<Layered>,
        pr: &Projector<'_>,
        quad: &[(f64, f64, f64); 4],
        color: [u8; 3],
    ) {
        let mut pts = Vec::with_capacity(4);
        let mut depth = 0.0;
        for &pt in quad {
            let (s, d) = pr(pt);
            pts.push(s);
            depth += d;
        }
        layers.push(Layered {
            depth: depth / 4.0,
            cmd: Cmd::Fill { pts, color },
        });
    }

    let outline = room.outline_or_rect();
    let cells = rect_cells(&outline);
    // Floor cells.
    for &(x1, y1, x2, y2) in &cells {
        push_quad(
            &mut layers,
            &pr,
            &[(x1, y1, 0.0), (x2, y1, 0.0), (x2, y2, 0.0), (x1, y2, 0.0)],
            shade(room.floor_rgb, 0.9),
        );
    }
    // Floor pattern lines, biased a hair nearer than the floor itself
    // so they paint over it but stay under furniture.
    let line_color = shade(room.floor_rgb, 0.7);
    for (a, b) in room.floor_material.pattern_segments(&cells) {
        let (sa, da) = pr((a.0, a.1, 0.0));
        let (sb, db) = pr((b.0, b.1, 0.0));
        layers.push(Layered {
            depth: (da + db) / 2.0 - 0.02,
            cmd: Cmd::Line {
                a: sa,
                b: sb,
                color: line_color,
            },
        });
    }
    // Walls: one quad per outline edge, shaded by facing.
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
        push_quad(
            &mut layers,
            &pr,
            &[
                (a.0, a.1, 0.0),
                (b.0, b.1, 0.0),
                (b.0, b.1, room.height_m),
                (a.0, a.1, room.height_m),
            ],
            shade(room.wall_rgb, bright),
        );
    }
    // Furniture boxes.
    for it in &p.items {
        let corners = box_corners(it);
        for (idx, bright) in box_faces() {
            let quad = [
                corners[idx[0]],
                corners[idx[1]],
                corners[idx[2]],
                corners[idx[3]],
            ];
            push_quad(&mut layers, &pr, &quad, shade(it.rgb, bright));
        }
    }
    // Window glass, painted last like the live view.
    let mut glass: Vec<Vec<(f64, f64)>> = Vec::new();
    for o in &p.openings {
        if o.kind != OpeningKind::Window {
            continue;
        }
        let quad = wall_opening_quad(p, o);
        glass.push(quad.iter().map(|&pt| pr(pt).0).collect());
    }

    layers.sort_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for layer in layers {
        match layer.cmd {
            Cmd::Fill { pts, color } => fill_polygon(&mut img, &pts, color),
            Cmd::Line { a, b, color } => draw_line(&mut img, a, b, color),
        }
    }
    for pts in glass {
        fill_polygon(&mut img, &pts, [120, 190, 240]);
    }
    img
}

fn fill_polygon(img: &mut RgbImage, pts: &[(f64, f64)], color: [u8; 3]) {
    // Fan triangulation (faces here are convex quads).
    for i in 1..pts.len().saturating_sub(1) {
        fill_triangle(img, pts[0], pts[i], pts[i + 1], color);
    }
}

fn fill_triangle(img: &mut RgbImage, a: (f64, f64), b: (f64, f64), c: (f64, f64), color: [u8; 3]) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let min_x = (a.0.min(b.0).min(c.0).floor() as i32).max(0);
    let max_x = (a.0.max(b.0).max(c.0).ceil() as i32).min(w - 1);
    let min_y = (a.1.min(b.1).min(c.1).floor() as i32).max(0);
    let max_y = (a.1.max(b.1).max(c.1).ceil() as i32).min(h - 1);
    let edge = |p: (f64, f64), p1: (f64, f64), p2: (f64, f64)| {
        (p.0 - p1.0) * (p2.1 - p1.1) - (p.1 - p1.1) * (p2.0 - p1.0)
    };
    let area = edge(a, b, c);
    if area.abs() < 1e-9 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f64 + 0.5, y as f64 + 0.5);
            let (e1, e2, e3) = (edge(p, a, b), edge(p, b, c), edge(p, c, a));
            let inside =
                (e1 >= 0.0 && e2 >= 0.0 && e3 >= 0.0) || (e1 <= 0.0 && e2 <= 0.0 && e3 <= 0.0);
            if inside {
                img.put_pixel(x as u32, y as u32, Rgb(color));
            }
        }
    }
}

fn draw_line(img: &mut RgbImage, a: (f64, f64), b: (f64, f64), color: [u8; 3]) {
    let (mut x0, mut y0) = (a.0.round() as i32, a.1.round() as i32);
    let (x1, y1) = (b.0.round() as i32, b.1.round() as i32);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && (x0 as u32) < img.width() && (y0 as u32) < img.height() {
            img.put_pixel(x0 as u32, y0 as u32, Rgb(color));
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FloorMaterial;

    #[test]
    fn snapshot_has_room_pixels_and_background() {
        let p = Project::default();
        let img = render_snapshot(&p, 320, 200);
        assert_eq!((img.width(), img.height()), (320, 200));
        // A corner stays background; somewhere in the middle the room is.
        assert_eq!(img.get_pixel(2, 2).0, [8, 8, 8]);
        let non_bg = img.pixels().filter(|px| px.0 != [8, 8, 8]).count();
        assert!(non_bg > 5_000, "only {non_bg} room pixels");
        // Deterministic.
        let img2 = render_snapshot(&p, 320, 200);
        assert!(img.as_raw() == img2.as_raw());
    }

    #[test]
    fn floor_pattern_changes_the_snapshot() {
        let mut p = Project::default();
        let plain = render_snapshot(&p, 320, 200);
        p.room.floor_material = FloorMaterial::CeramicTile;
        p.room.floor_rgb = FloorMaterial::CeramicTile.base_rgb().unwrap();
        let tiled = render_snapshot(&p, 320, 200);
        assert!(plain.as_raw() != tiled.as_raw());
    }
}
