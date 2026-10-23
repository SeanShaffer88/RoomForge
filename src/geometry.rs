//! Small 2D geometry helpers for oriented rectangles (furniture with a
//! rotation). Rectangles are given as four corners in order.

/// Separating-axis test: do two oriented rectangles overlap?
/// Touching edges do not count as overlapping.
pub fn obb_overlap(a: &[(f64, f64); 4], b: &[(f64, f64); 4]) -> bool {
    for rect in [a, b] {
        for i in 0..2 {
            // Two axes per rectangle are enough (the other two are parallel).
            let (x1, y1) = rect[i];
            let (x2, y2) = rect[i + 1];
            let axis = (-(y2 - y1), x2 - x1);
            let (amin, amax) = project(a, axis);
            let (bmin, bmax) = project(b, axis);
            if amax <= bmin + 1e-9 || bmax <= amin + 1e-9 {
                return false;
            }
        }
    }
    true
}

fn project(rect: &[(f64, f64); 4], axis: (f64, f64)) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &(x, y) in rect {
        let d = x * axis.0 + y * axis.1;
        min = min.min(d);
        max = max.max(d);
    }
    (min, max)
}

fn project_poly(poly: &[(f64, f64)], axis: (f64, f64)) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &(x, y) in poly {
        let d = x * axis.0 + y * axis.1;
        min = min.min(d);
        max = max.max(d);
    }
    (min, max)
}

/// Separating-axis test for two convex polygons (vertices in order).
/// Touching edges do not count as overlapping. Used for door-swing
/// sectors vs furniture footprints.
pub fn convex_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    for poly in [a, b] {
        for i in 0..poly.len() {
            let (x1, y1) = poly[i];
            let (x2, y2) = poly[(i + 1) % poly.len()];
            let axis = (-(y2 - y1), x2 - x1);
            if axis == (0.0, 0.0) {
                continue;
            }
            let (amin, amax) = project_poly(a, axis);
            let (bmin, bmax) = project_poly(b, axis);
            if amax <= bmin + 1e-9 || bmax <= amin + 1e-9 {
                return false;
            }
        }
    }
    true
}

/// Is a point inside an oriented rectangle (inclusive of the edge)?
pub fn point_in_obb(px: f64, py: f64, rect: &[(f64, f64); 4]) -> bool {
    // Same-sign cross products for a consistently wound polygon.
    let mut sign = 0i32;
    for i in 0..4 {
        let (x1, y1) = rect[i];
        let (x2, y2) = rect[(i + 1) % 4];
        let cross = (x2 - x1) * (py - y1) - (y2 - y1) * (px - x1);
        let s = if cross > 1e-9 {
            1
        } else if cross < -1e-9 {
            -1
        } else {
            0
        };
        if s != 0 {
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}

/// Area of a simple polygon (shoelace formula).
pub fn polygon_area(pts: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        sum += x1 * y2 - x2 * y1;
    }
    (sum / 2.0).abs()
}

/// Perimeter of a polygon.
pub fn polygon_perimeter(pts: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        sum += ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    }
    sum
}

/// Is a point inside a simple polygon? Points on the boundary count
/// as inside. Ray casting, even–odd rule.
pub fn point_in_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
    // Boundary check first (within a hair of any edge).
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len2 = dx * dx + dy * dy;
        if len2 > 0.0 {
            let t = (((px - x1) * dx + (py - y1) * dy) / len2).clamp(0.0, 1.0);
            let (cx, cy) = (x1 + t * dx, y1 + t * dy);
            if (px - cx).powi(2) + (py - cy).powi(2) < 1e-12 {
                return true;
            }
        }
    }
    let mut inside = false;
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        if (y1 > py) != (y2 > py) {
            let xint = x1 + (py - y1) / (y2 - y1) * (x2 - x1);
            if px < xint {
                inside = !inside;
            }
        }
    }
    inside
}

/// Area-weighted centroid of a simple polygon.
pub fn polygon_centroid(pts: &[(f64, f64)]) -> (f64, f64) {
    let (mut cx, mut cy, mut a2) = (0.0, 0.0, 0.0);
    for i in 0..pts.len() {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % pts.len()];
        let f = x1 * y2 - x2 * y1;
        a2 += f;
        cx += (x1 + x2) * f;
        cy += (y1 + y2) * f;
    }
    if a2.abs() < 1e-12 {
        return (0.0, 0.0);
    }
    (cx / (3.0 * a2), cy / (3.0 * a2))
}

/// Decompose a rectilinear polygon into rectangles: take every
/// distinct vertex x and y as a grid; a cell belongs to the polygon
/// when its centre is inside. Used to fill concave floors in the 2D
/// and 3D views. Returns (x1, y1, x2, y2) cells.
pub fn rect_cells(pts: &[(f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
    let mut xs: Vec<f64> = pts.iter().map(|p| p.0).collect();
    let mut ys: Vec<f64> = pts.iter().map(|p| p.1).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let mut out = Vec::new();
    for i in 0..xs.len().saturating_sub(1) {
        for j in 0..ys.len().saturating_sub(1) {
            let (cx, cy) = ((xs[i] + xs[i + 1]) / 2.0, (ys[j] + ys[j + 1]) / 2.0);
            if point_in_polygon(cx, cy, pts) {
                out.push((xs[i], ys[j], xs[i + 1], ys[j + 1]));
            }
        }
    }
    out
}

/// Snap a value to the nearest step (used for 5 cm placement snapping).
pub fn snap(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        value
    } else {
        (value / step).round() * step
    }
}

/// The result of alignment snapping while dragging: an adjustment
/// to apply to the moving item's centre, and the room-coordinate
/// lines (vertical x's, horizontal y's) that should light up as
/// guides because something aligned there.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlignSnap {
    pub dx: f64,
    pub dy: f64,
    pub guides_x: Vec<f64>,
    pub guides_y: Vec<f64>,
}

/// Snap a moving item's bounding box to its neighbours and the
/// room: edges and centres of other items, the walls, and the
/// room's centre lines — whichever alignment is closest within
/// `thresh` metres (per axis). This is the "magnetic" feel of a
/// polished planner, as a pure function.
pub fn align_snap(
    moving: (f64, f64, f64, f64),
    others: &[(f64, f64, f64, f64)],
    room_w: f64,
    room_d: f64,
    thresh: f64,
) -> AlignSnap {
    fn axis(
        feats: [f64; 3],
        mut targets: Vec<f64>,
        others: &[(f64, f64, f64, f64)],
        y_axis: bool,
        thresh: f64,
    ) -> (f64, Vec<f64>) {
        for o in others {
            let (lo, hi) = if y_axis { (o.1, o.3) } else { (o.0, o.2) };
            targets.push(lo);
            targets.push((lo + hi) / 2.0);
            targets.push(hi);
        }
        let mut best: Option<(f64, f64)> = None; // (abs delta, delta)
        for f in feats {
            for t in &targets {
                let d = t - f;
                if d.abs() <= thresh && best.map(|(b, _)| d.abs() < b - 1e-12).unwrap_or(true) {
                    best = Some((d.abs(), d));
                }
            }
        }
        match best {
            None => (0.0, Vec::new()),
            Some((_, d)) => {
                let guides = targets
                    .iter()
                    .copied()
                    .filter(|t| feats.iter().any(|f| (t - (f + d)).abs() < 1e-9))
                    .collect();
                (d, guides)
            }
        }
    }
    let (dx, guides_x) = axis(
        [moving.0, (moving.0 + moving.2) / 2.0, moving.2],
        vec![0.0, room_w / 2.0, room_w],
        others,
        false,
        thresh,
    );
    let (dy, guides_y) = axis(
        [moving.1, (moving.1 + moving.3) / 2.0, moving.3],
        vec![0.0, room_d / 2.0, room_d],
        others,
        true,
        thresh,
    );
    AlignSnap {
        dx,
        dy,
        guides_x,
        guides_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_snap_finds_edges_centres_and_room_lines() {
        // Other item occupies x 1.0..2.0, y 1.0..1.5 in a 5x4 room.
        let others = [(1.0, 1.0, 2.0, 1.5)];
        // Moving item (1x1) with its left edge 3 cm from the other's:
        let moving = (2.03, 3.0, 3.03, 4.0);
        let s = align_snap(moving, &others, 5.0, 4.0, 0.08);
        assert!((s.dx + 0.03).abs() < 1e-9, "dx {}", s.dx);
        assert!(s.guides_x.contains(&2.0), "guides {:?}", s.guides_x);
        // Centre-to-centre: moving centre x = 1.47 vs other centre 1.5.
        let moving2 = (0.97, 3.0, 1.97, 4.0);
        let s2 = align_snap(moving2, &others, 5.0, 4.0, 0.08);
        assert!((s2.dx - 0.03).abs() < 1e-9 && s2.guides_x.contains(&1.5));
        // Room centre line: centre at 2.47 → snaps to 2.5.
        let s3 = align_snap((1.97, 0.2, 2.97, 1.2), &[], 5.0, 4.0, 0.08);
        assert!((s3.dx - 0.03).abs() < 1e-9 && s3.guides_x.contains(&2.5));
        // Beyond the threshold: nothing happens.
        let s4 = align_snap((2.2, 3.0, 3.2, 4.0), &others, 5.0, 4.0, 0.08);
        // (left edge 2.2 is 0.2 from 2.0; centre 2.7 is 0.2 from 2.5 room centre)
        assert_eq!(s4.dx, 0.0);
        assert!(s4.guides_x.is_empty());
    }

    fn rect(cx: f64, cy: f64, w: f64, d: f64) -> [(f64, f64); 4] {
        [
            (cx - w / 2.0, cy - d / 2.0),
            (cx + w / 2.0, cy - d / 2.0),
            (cx + w / 2.0, cy + d / 2.0),
            (cx - w / 2.0, cy + d / 2.0),
        ]
    }

    #[test]
    fn separate_rects_do_not_overlap() {
        assert!(!obb_overlap(
            &rect(0.0, 0.0, 1.0, 1.0),
            &rect(3.0, 0.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn touching_rects_do_not_overlap() {
        assert!(!obb_overlap(
            &rect(0.0, 0.0, 1.0, 1.0),
            &rect(1.0, 0.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn crossing_rects_overlap() {
        assert!(obb_overlap(
            &rect(0.0, 0.0, 2.0, 1.0),
            &rect(0.5, 0.0, 1.0, 2.0)
        ));
    }

    #[test]
    fn rotated_rect_overlap_is_detected() {
        // A 45° square whose corner reaches into the axis-aligned one.
        let c = std::f64::consts::FRAC_1_SQRT_2;
        let rotated = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];
        let _ = c;
        assert!(obb_overlap(&rect(1.2, 0.0, 1.0, 1.0), &rotated));
        assert!(!obb_overlap(&rect(2.2, 0.0, 1.0, 1.0), &rotated));
    }

    #[test]
    fn point_inside_and_outside() {
        let r = rect(1.0, 1.0, 2.0, 2.0);
        assert!(point_in_obb(1.5, 1.5, &r));
        assert!(!point_in_obb(3.5, 1.5, &r));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn convex_sector_overlap() {
        // Quarter-disc sector in the NE quadrant from the origin.
        let sector = vec![(0.0, 0.0), (1.0, 0.0), (0.7071, 0.7071), (0.0, 1.0)];
        let inside = vec![(0.4, 0.1), (0.7, 0.1), (0.7, 0.4), (0.4, 0.4)];
        let outside = vec![(1.4, 0.1), (1.7, 0.1), (1.7, 0.4), (1.4, 0.4)];
        assert!(convex_overlap(&sector, &inside));
        assert!(!convex_overlap(&sector, &outside));
    }

    #[test]
    fn polygon_measures_for_an_l_shape() {
        // 5 x 4 rectangle with a 2 x 1.5 notch cut from the NE corner.
        let l = vec![
            (0.0, 0.0),
            (5.0, 0.0),
            (5.0, 2.5),
            (3.0, 2.5),
            (3.0, 4.0),
            (0.0, 4.0),
        ];
        assert!((polygon_area(&l) - (20.0 - 3.0)).abs() < 1e-9);
        assert!((polygon_perimeter(&l) - 18.0).abs() < 1e-9);
        assert!(point_in_polygon(1.0, 1.0, &l));
        assert!(point_in_polygon(4.0, 1.0, &l));
        assert!(!point_in_polygon(4.0, 3.5, &l)); // inside the notch
        assert!(point_in_polygon(5.0, 1.0, &l)); // boundary counts
        let cells = rect_cells(&l);
        let cell_area: f64 = cells.iter().map(|c| (c.2 - c.0) * (c.3 - c.1)).sum();
        assert!((cell_area - 17.0).abs() < 1e-9, "cells: {cells:?}");
        let (cx, cy) = polygon_centroid(&l);
        assert!(point_in_polygon(cx, cy, &l));
    }

    #[test]
    fn snap_rounds_to_step() {
        assert!((snap(1.03, 0.05) - 1.05).abs() < 1e-9);
        assert!((snap(1.02, 0.05) - 1.0).abs() < 1e-9);
        assert_eq!(snap(1.02, 0.0), 1.02);
    }
}
