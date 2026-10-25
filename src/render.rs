//! Offline snapshot renderer: paints the 3D view into an image with a
//! small software rasterizer (painter's algorithm, far to near). No
//! GPU, no window — so a plan can be shared as a picture from
//! anywhere, and the renderer is testable off-Windows.

use image::{Rgb, RgbImage};

use crate::geometry::rect_cells;
use crate::model::{OpeningKind, Project};
use crate::view3d::{
    box_corners, box_faces, project, shade_scene, shadow_quad, skirting_quad, style_rgb,
    wall_opening_quad, Camera, Scene, Style, WallCut,
};

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
/// image of the given pixel size, in daylight.
pub fn render_snapshot(p: &Project, width: u32, height: u32) -> RgbImage {
    render_snapshot_scene(p, width, height, Scene::Day)
}

/// Render the snapshot under a lighting scene (day / evening / night).
/// Parse every imported model in the project. Snapshot renders are
/// one-off, so nothing is cached; a def that fails to parse is
/// simply absent and its items fall back to boxes.
pub fn prepared_models(
    p: &Project,
) -> std::collections::HashMap<u64, crate::model3d::PreparedModel> {
    p.custom_models
        .iter()
        .filter_map(|d| crate::model3d::prepare(d).ok().map(|m| (d.id, m)))
        .collect()
}

pub fn render_snapshot_scene(p: &Project, width: u32, height: u32, scene: Scene) -> RgbImage {
    render_snapshot_full(
        p,
        width,
        height,
        scene,
        Style::Coloured,
        WallCut::Full,
        false,
        0.0,
    )
}

/// The snapshot with a render style, a wall cut, optional open
/// doors/drawers and an exploded-view amount applied.
#[allow(clippy::too_many_arguments)]
pub fn render_snapshot_full(
    p: &Project,
    width: u32,
    height: u32,
    scene: Scene,
    style: Style,
    cut: WallCut,
    open_fronts: bool,
    explode: f64,
) -> RgbImage {
    let mut img = RgbImage::from_pixel(width, height, Rgb(scene.background()));
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
        push_quad_edged(layers, pr, quad, color, false);
    }

    fn push_quad_edged(
        layers: &mut Vec<Layered>,
        pr: &Projector<'_>,
        quad: &[(f64, f64, f64); 4],
        color: [u8; 3],
        edges: bool,
    ) {
        let mut pts = Vec::with_capacity(4);
        let mut depth = 0.0;
        for &pt in quad {
            let (s, d) = pr(pt);
            pts.push(s);
            depth += d;
        }
        let depth = depth / 4.0;
        layers.push(Layered {
            depth,
            cmd: Cmd::Fill {
                pts: pts.clone(),
                color,
            },
        });
        if edges {
            // Technical-drawing look: a dark line on every edge,
            // biased a hair nearer so it paints over its own fill.
            for i in 0..4 {
                layers.push(Layered {
                    depth: depth - 0.015,
                    cmd: Cmd::Line {
                        a: pts[i],
                        b: pts[(i + 1) % 4],
                        color: [52, 50, 48],
                    },
                });
            }
        }
    }

    // A quad pushed with an artificial depth bias, for flat floor
    // dressing (shadows) that must paint over the floor + pattern.
    fn push_biased(
        layers: &mut Vec<Layered>,
        pr: &Projector<'_>,
        quad: &[(f64, f64, f64); 4],
        color: [u8; 3],
        bias: f64,
    ) {
        let mut tmp = Vec::new();
        push_quad(&mut tmp, pr, quad, color);
        for mut l in tmp {
            l.depth -= bias;
            layers.push(l);
        }
    }

    let outline = room.outline_or_rect();
    let cells = rect_cells(&outline);
    let top = cut.cut_height(room.height_m);
    let floor_rgb = style_rgb(room.floor_rgb, style);
    let wall_rgb = style_rgb(room.wall_rgb, style);
    // Floor cells.
    for &(x1, y1, x2, y2) in &cells {
        push_quad_edged(
            &mut layers,
            &pr,
            &[(x1, y1, 0.0), (x2, y1, 0.0), (x2, y2, 0.0), (x1, y2, 0.0)],
            shade_scene(floor_rgb, 0.9, scene),
            style == Style::Outline,
        );
    }
    // Floor pattern lines, biased a hair nearer than the floor itself
    // so they paint over it but stay under furniture.
    let line_color = shade_scene(floor_rgb, 0.7, scene);
    let pattern_segs = if style == Style::Outline {
        Vec::new()
    } else {
        room.floor_material.pattern_segments(&cells)
    };
    for (a, b) in pattern_segs {
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
        push_quad_edged(
            &mut layers,
            &pr,
            &[
                (a.0, a.1, 0.0),
                (b.0, b.1, 0.0),
                (b.0, b.1, top),
                (a.0, a.1, top),
            ],
            shade_scene(wall_rgb, bright, scene),
            style == Style::Outline,
        );
    }
    // Skirting boards along every wall base.
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        push_quad(
            &mut layers,
            &pr,
            &skirting_quad(a, b),
            shade_scene(wall_rgb, 1.18, scene),
        );
    }
    // Soft shadows under the furniture (over floor and pattern).
    for it in &p.items {
        let it = &crate::view3d::exploded(it, centre, explode);
        if let Some(q) = shadow_quad(it) {
            push_biased(
                &mut layers,
                &pr,
                &q,
                shade_scene(floor_rgb, 0.55, scene),
                0.04,
            );
        }
    }
    // Furniture: imported models as real geometry, the rest boxes
    // with front panels. The exploded view shifts every piece
    // (and its shadow, above) away from the room centre.
    let models = prepared_models(p);
    for it in &p.items {
        let it = &crate::view3d::exploded(it, centre, explode);
        if let Some(prep) = crate::model::model_id_of(&it.catalog_id).and_then(|id| models.get(&id))
        {
            for (tri, bright, rgb) in crate::view3d::mesh_tris(it, prep) {
                push_quad_edged(
                    &mut layers,
                    &pr,
                    &[tri[0], tri[1], tri[2], tri[2]],
                    shade_scene(style_rgb(rgb, style), bright, scene),
                    style == Style::Outline,
                );
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
            push_quad_edged(
                &mut layers,
                &pr,
                &quad,
                shade_scene(style_rgb(it.rgb, style), bright, scene),
                style == Style::Outline,
            );
        }
        // Front panels (doors, drawers), open or closed.
        for (quad, bright, rgb) in
            crate::view3d::front_quads(it, if open_fronts { 1.0 } else { 0.0 })
        {
            push_quad_edged(
                &mut layers,
                &pr,
                &quad,
                shade_scene(style_rgb(rgb, style), bright, scene),
                style == Style::Outline,
            );
        }
    }
    // Window glass, painted last like the live view. A wall cut
    // clips the glass at the cut height; a window whose sill is
    // above the cut disappears with its wall.
    let mut glass: Vec<Vec<(f64, f64)>> = Vec::new();
    for o in &p.openings {
        if o.kind != OpeningKind::Window || o.sill_m >= top {
            continue;
        }
        let quad = wall_opening_quad(p, o);
        glass.push(
            quad.iter()
                .map(|&pt| pr((pt.0, pt.1, pt.2.min(top))).0)
                .collect(),
        );
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
        fill_polygon(&mut img, &pts, scene.glass());
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

/// Render the first-person walk view to an image — the "photo from
/// where you're standing" companion to the orbit snapshot.
pub fn render_walk(
    p: &Project,
    w: &crate::walk::Walker,
    scene: Scene,
    style: Style,
    open_fronts: bool,
    width: u32,
    height: u32,
) -> RgbImage {
    let mut img = RgbImage::from_pixel(width, height, Rgb(scene.background()));
    if width == 0 || height == 0 {
        return img;
    }
    let models = prepared_models(p);
    for layer in crate::walk::walk_layers(
        p,
        w,
        scene,
        style,
        open_fronts,
        width as f64,
        height as f64,
        &models,
    ) {
        match layer.cmd {
            crate::walk::WalkCmd::Fill { pts, color } => fill_polygon(&mut img, &pts, color),
            crate::walk::WalkCmd::Line { a, b, color } => draw_line(&mut img, a, b, color),
        }
    }
    img
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
    fn night_snapshot_is_darker_with_a_night_backdrop() {
        let p = Project::default();
        let day = render_snapshot(&p, 320, 200);
        let night = render_snapshot_scene(&p, 320, 200, Scene::Night);
        assert_eq!(night.get_pixel(2, 2).0, Scene::Night.background());
        let mean = |img: &RgbImage| -> f64 {
            let px: Vec<[u8; 3]> = img
                .pixels()
                .map(|q| q.0)
                .filter(|c| *c != Scene::Day.background() && *c != Scene::Night.background())
                .collect();
            px.iter()
                .map(|c| c[0] as f64 + c[1] as f64 + c[2] as f64)
                .sum::<f64>()
                / (px.len() as f64 * 3.0)
        };
        assert!(
            mean(&night) < mean(&day) * 0.75,
            "night mean {} vs day {}",
            mean(&night),
            mean(&day)
        );
    }

    #[test]
    fn snapshot_renders_imported_model_geometry() {
        use base64::Engine;
        // A 1 m cube as OBJ, bright red.
        let obj = "v 0 0 0\nv 1 0 0\nv 1 0 1\nv 0 0 1\nv 0 1 0\nv 1 1 0\nv 1 1 1\nv 0 1 1\n                   f 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 4 8 5 1\n";
        let mut p = Project::default();
        let def_id = p.alloc_id();
        p.custom_models.push(crate::model::CustomModelDef {
            id: def_id,
            name: "Cube".into(),
            format: "obj".into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(obj.as_bytes()),
            unit_depth: 1.0,
            unit_height: 1.0,
            natural_width_m: 1.0,
        });
        let item_id = p.alloc_id();
        p.items.push(crate::model::PlacedItem {
            id: item_id,
            catalog_id: format!("model-{def_id}"),
            name: "Cube".into(),
            category: "3D models".into(),
            x_m: 2.5,
            y_m: 2.0,
            width_m: 1.0,
            depth_m: 1.0,
            height_m: 1.0,
            rotation_deg: 0.0,
            rgb: [200, 30, 30],
        });
        let img = render_snapshot(&p, 320, 200);
        // The model's own colour (the OBJ default grey, shaded)
        // must appear: grey ~168 shaded by 0.5–1.0 → look for
        // mid-grey pixels that are NOT the room's palette.
        let greyish = img
            .pixels()
            .filter(|px| {
                let [r, g, b] = px.0;
                r > 80
                    && r < 180
                    && (r as i32 - g as i32).abs() < 8
                    && (g as i32 - b as i32).abs() < 8
            })
            .count();
        assert!(greyish > 50, "model faces visible (got {greyish})");
        // Remove the def: the item falls back to its red box.
        p.custom_models.clear();
        let img2 = render_snapshot(&p, 320, 200);
        let reds = img2
            .pixels()
            .filter(|px| px.0[0] > 120 && px.0[1] < 90 && px.0[2] < 90)
            .count();
        assert!(reds > 50, "box fallback renders (got {reds})");
    }

    #[test]
    fn open_fronts_and_explode_change_the_snapshot() {
        let mut p = Project::default();
        for (id, x, y) in [("wardrobe-2", 1.2, 1.0), ("chest-4", 3.8, 3.0)] {
            let iid = p.alloc_id();
            p.items.push(crate::model::PlacedItem {
                id: iid,
                catalog_id: id.into(),
                name: id.into(),
                category: "Storage".into(),
                x_m: x,
                y_m: y,
                width_m: 1.2,
                depth_m: 0.6,
                height_m: 2.0,
                rotation_deg: 0.0,
                rgb: [120, 90, 60],
            });
        }
        let diffs = |a: &RgbImage, b: &RgbImage| {
            a.pixels()
                .zip(b.pixels())
                .filter(|(x, y)| x.0 != y.0)
                .count()
        };
        let closed = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Coloured,
            WallCut::Full,
            false,
            0.0,
        );
        let open = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Coloured,
            WallCut::Full,
            true,
            0.0,
        );
        assert!(
            diffs(&closed, &open) > 100,
            "open fronts change the picture"
        );
        let boom = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Coloured,
            WallCut::Full,
            false,
            1.0,
        );
        assert!(
            diffs(&closed, &boom) > 500,
            "exploded view moves the furniture"
        );
    }

    #[test]
    fn styled_and_cut_snapshots_differ() {
        // Strongly coloured room, so clay's greying is measurable.
        let mut p = Project::default();
        p.room.floor_rgb = [200, 30, 30];
        p.room.wall_rgb = [30, 30, 200];
        let grey_frac = |img: &RgbImage| {
            let grey = img
                .pixels()
                .filter(|px| {
                    let (r, g, b) = (px.0[0] as i32, px.0[1] as i32, px.0[2] as i32);
                    (r - g).abs() < 14 && (g - b).abs() < 18
                })
                .count();
            grey as f64 / (img.width() * img.height()) as f64
        };
        let coloured = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Coloured,
            WallCut::Full,
            false,
            0.0,
        );
        let clay = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Clay,
            WallCut::Full,
            false,
            0.0,
        );
        assert!(
            grey_frac(&clay) > grey_frac(&coloured) + 0.15,
            "clay should be much greyer: clay {:.2} vs coloured {:.2}",
            grey_frac(&clay),
            grey_frac(&coloured)
        );
        let low = render_snapshot_full(
            &p,
            320,
            200,
            Scene::Day,
            Style::Coloured,
            WallCut::Low,
            false,
            0.0,
        );
        let diff = coloured
            .pixels()
            .zip(low.pixels())
            .filter(|(a, b)| a.0 != b.0)
            .count();
        assert!(
            diff > 500,
            "a low wall cut changes the picture (got {diff})"
        );
    }

    #[test]
    fn walk_render_shows_the_room_from_inside() {
        let p = Project::default();
        let w = crate::walk::Walker::start(&p);
        let img = render_walk(&p, &w, Scene::Day, Style::Coloured, false, 320, 200);
        // Standing inside, the room fills essentially the whole frame.
        let non_bg = img
            .pixels()
            .filter(|px| px.0 != Scene::Day.background())
            .count();
        assert!(non_bg > 30_000, "only {non_bg} room pixels from inside");
        let night = render_walk(&p, &w, Scene::Night, Style::Coloured, false, 320, 200);
        assert!(night.as_raw() != img.as_raw());
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
