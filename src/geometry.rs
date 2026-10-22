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

/// Snap a value to the nearest step (used for 5 cm placement snapping).
pub fn snap(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        value
    } else {
        (value / step).round() * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn snap_rounds_to_step() {
        assert!((snap(1.03, 0.05) - 1.05).abs() < 1e-9);
        assert!((snap(1.02, 0.05) - 1.0).abs() < 1e-9);
        assert_eq!(snap(1.02, 0.0), 1.02);
    }
}
