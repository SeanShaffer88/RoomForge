//! Exports: an SVG floor plan, a CSV item list, and a printable HTML
//! sheet (plan + item list). Everything is generated from the project
//! model alone so it can be tested off-Windows. No prices — by design.

use crate::model::{OpeningKind, Project, Wall};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Where an opening sits in plan coordinates: returns the (x, y) of its
/// start corner and its direction along the wall.
fn opening_segment(p: &Project, o: &crate::model::Opening) -> ((f64, f64), (f64, f64)) {
    let (w, d) = (p.room.width_m, p.room.depth_m);
    match o.wall {
        Wall::South => ((o.offset_m, 0.0), (o.offset_m + o.width_m, 0.0)),
        Wall::North => ((o.offset_m, d), (o.offset_m + o.width_m, d)),
        Wall::West => ((0.0, o.offset_m), (0.0, o.offset_m + o.width_m)),
        Wall::East => ((w, o.offset_m), (w, o.offset_m + o.width_m)),
    }
}

/// A top-down SVG floor plan, in metres mapped at `px_per_m`.
pub fn plan_svg(p: &Project, px_per_m: f64) -> String {
    let (w, d, h) = (p.room.width_m, p.room.depth_m, p.room.height_m);
    let margin = 60.0;
    let width_px = w * px_per_m + margin * 2.0;
    let height_px = d * px_per_m + margin * 2.0;
    // Plan coordinates: x east → right, y north → up (SVG y grows down).
    let sx = |x: f64| margin + x * px_per_m;
    let sy = |y: f64| margin + (d - y) * px_per_m;
    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width_px:.0}\" height=\"{height_px:.0}\" viewBox=\"0 0 {width_px:.0} {height_px:.0}\">\n"
    ));
    s.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n");
    // Floor — the outline stroke is the wall, drawn at its real thickness.
    let wall_px = (p.room.wall_thickness_m * px_per_m).max(2.0);
    s.push_str(&format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\" stroke=\"#222\" stroke-width=\"{wall_px:.1}\"/>\n",
        sx(0.0),
        sy(d),
        w * px_per_m,
        d * px_per_m,
        rgb_hex(p.room.floor_rgb)
    ));
    // Dimension lines: width below the south wall, depth left of the west wall.
    let dim_s = sy(0.0) + 30.0;
    s.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\" fill=\"#222\">{}</text>\n",
        sx(0.0), dim_s - 5.0, sx(0.0), dim_s + 5.0,
        sx(w), dim_s - 5.0, sx(w), dim_s + 5.0,
        sx(0.0), dim_s, sx(w), dim_s,
        sx(w / 2.0), dim_s - 6.0,
        crate::units::fmt_len(w, false)
    ));
    let dim_w = sx(0.0) - 30.0;
    s.push_str(&format!(
        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"1\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\" fill=\"#222\" transform=\"rotate(-90 {:.1} {:.1})\">{}</text>\n",
        dim_w - 5.0, sy(0.0), dim_w + 5.0, sy(0.0),
        dim_w - 5.0, sy(d), dim_w + 5.0, sy(d),
        dim_w, sy(0.0), dim_w, sy(d),
        dim_w - 6.0, sy(d / 2.0), dim_w - 6.0, sy(d / 2.0),
        crate::units::fmt_len(d, false)
    ));
    // Dimension labels
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" text-anchor=\"middle\" fill=\"#222\">{} — {:.2} m wide × {:.2} m deep × {:.2} m ceiling</text>\n",
        width_px / 2.0,
        24.0,
        esc(&p.room.name),
        w,
        d,
        h
    ));
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"12\" text-anchor=\"middle\" fill=\"#222\">Floor area {:.1} m²</text>\n",
        width_px / 2.0,
        height_px - 16.0,
        p.room.area_m2()
    ));
    // Openings: doors as a gap + swing arc, windows as a blue double line.
    for o in &p.openings {
        let ((x1, y1), (x2, y2)) = opening_segment(p, o);
        match o.kind {
            OpeningKind::Window => {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#2b6cb0\" stroke-width=\"7\"/>\n",
                    sx(x1),
                    sy(y1),
                    sx(x2),
                    sy(y2)
                ));
            }
            OpeningKind::Door => {
                s.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#ffffff\" stroke-width=\"5\"/>\n",
                    sx(x1),
                    sy(y1),
                    sx(x2),
                    sy(y2)
                ));
                if o.swings_in {
                    // Leaf at 90° into the room from the hinge (start corner).
                    let (hx, hy) = (x1, y1);
                    let inward: (f64, f64) = match o.wall {
                        Wall::South => (hx, hy + o.width_m),
                        Wall::North => (hx, hy - o.width_m),
                        Wall::West => (hx + o.width_m, hy),
                        Wall::East => (hx - o.width_m, hy),
                    };
                    s.push_str(&format!(
                        "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#222\" stroke-width=\"2\"/>\n",
                        sx(hx),
                        sy(hy),
                        sx(inward.0),
                        sy(inward.1)
                    ));
                }
            }
        }
    }
    // Services
    for svc in &p.services {
        let (cx, cy) = svc.centre(&p.room);
        let (label, rgb) = svc.kind.symbol();
        let fill = rgb_hex(rgb);
        let fsize = if label.len() > 1 { 8 } else { 10 };
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"9\" fill=\"{fill}\" stroke=\"#222\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"{fsize}\" text-anchor=\"middle\" fill=\"#111\">{label}</text>\n",
            sx(cx),
            sy(cy),
            sx(cx),
            sy(cy) + 3.5
        ));
    }
    // Furniture
    for it in &p.items {
        let corners = it.corners();
        let pts: Vec<String> = corners
            .iter()
            .map(|&(x, y)| format!("{:.1},{:.1}", sx(x), sy(y)))
            .collect();
        s.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"0.85\" stroke=\"#222\" stroke-width=\"1.5\"/>\n",
            pts.join(" "),
            rgb_hex(it.rgb)
        ));
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" text-anchor=\"middle\" fill=\"#111\">{}</text>\n",
            sx(it.x_m),
            sy(it.y_m) + 4.0,
            esc(&it.name)
        ));
    }
    // Annotations: a circled number marker with the note text beside it.
    for (i, a) in p.annotations.iter().enumerate() {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"8\" fill=\"#e5484d\" stroke=\"#fff\"/>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"10\" text-anchor=\"middle\" fill=\"#fff\">{}</text>\n<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"#222\">{}</text>\n",
            sx(a.x_m),
            sy(a.y_m),
            sx(a.x_m),
            sy(a.y_m) + 3.5,
            i + 1,
            sx(a.x_m) + 12.0,
            sy(a.y_m) + 4.0,
            esc(&a.text)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// The plan SVG at a true print scale (1:50, 1:100, …): the root
/// element carries physical millimetre dimensions, so printing the
/// file at 100% reproduces the scale exactly.
pub fn plan_svg_scaled(p: &Project, denominator: f64) -> String {
    let px_per_m = 100.0;
    let svg = plan_svg(p, px_per_m);
    // 100 units = 1 m = 1000 mm at full size; at 1:denominator one
    // unit is 10/denominator mm on paper.
    let k = 10.0 / denominator;
    let wp = p.room.width_m * px_per_m + 120.0;
    let hp = p.room.depth_m * px_per_m + 120.0;
    svg.replacen(
        &format!("width=\"{wp:.0}\" height=\"{hp:.0}\""),
        &format!("width=\"{:.1}mm\" height=\"{:.1}mm\"", wp * k, hp * k),
        1,
    )
}

/// CSV of the grouped item list. Columns: Item, Category, Width m,
/// Depth m, Height m, Quantity. No prices — by design.
pub fn list_csv(p: &Project) -> String {
    let mut out = String::from("Item,Category,Width (m),Depth (m),Height (m),Quantity\n");
    for (name, cat, w, d, h, n) in p.item_list() {
        let name = if name.contains(',') {
            format!("\"{name}\"")
        } else {
            name
        };
        out.push_str(&format!("{name},{cat},{w:.2},{d:.2},{h:.2},{n}\n"));
    }
    out
}

/// A self-contained printable HTML page: the SVG plan, the item list,
/// and the openings/services schedule. Open it in any browser and print
/// (or print to PDF).
pub fn print_html(p: &Project) -> String {
    let svg = plan_svg(p, 90.0);
    let mut rows = String::new();
    for (name, cat, w, d, h, n) in p.item_list() {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.2} × {:.2} × {:.2} m</td><td>{}</td></tr>",
            esc(&name),
            esc(&cat),
            w,
            d,
            h,
            n
        ));
    }
    if rows.is_empty() {
        rows = "<tr><td colspan=\"4\">No furniture placed yet.</td></tr>".to_string();
    }
    let mut fittings = String::new();
    for o in &p.openings {
        fittings.push_str(&format!(
            "<li>{} — {} wall, {:.2} m from the corner, {:.2} m wide</li>",
            o.kind.label(),
            o.wall.label(),
            o.offset_m,
            o.width_m
        ));
    }
    for svc in &p.services {
        fittings.push_str(&format!(
            "<li>{} — {} wall, {:.2} m from the corner</li>",
            svc.kind.label(),
            svc.wall.label(),
            svc.offset_m
        ));
    }
    if fittings.is_empty() {
        fittings = "<li>None recorded.</li>".to_string();
    }
    let mut notes = String::new();
    for (i, a) in p.annotations.iter().enumerate() {
        notes.push_str(&format!("<li>{}. {}</li>", i + 1, esc(&a.text)));
    }
    let notes_section = if notes.is_empty() {
        String::new()
    } else {
        format!("<h2>Notes on the plan</h2><ol>{notes}</ol>")
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{} — RoomForge plan</title>\n<style>body{{font-family:Segoe UI,Arial,sans-serif;margin:24px;color:#111}}table{{border-collapse:collapse;margin-top:12px}}td,th{{border:1px solid #999;padding:4px 10px;text-align:left}}h1{{font-size:20px}}h2{{font-size:15px;margin-top:24px}}@media print{{body{{margin:8px}}}}</style></head>\n<body><h1>{} — {:.2} m × {:.2} m (ceiling {:.2} m)</h1>\n{}\n<h2>Item list</h2><table><tr><th>Item</th><th>Category</th><th>Size (W × D × H)</th><th>Qty</th></tr>{}</table>\n<h2>Doors, windows and services</h2><ul>{}</ul>{}\n<p>Drawn to scale by RoomForge. E = socket, W = pipe/water, R = radiator.</p></body></html>\n",
        esc(&p.room.name),
        esc(&p.room.name),
        p.room.width_m,
        p.room.depth_m,
        p.room.height_m,
        svg,
        rows,
        fittings,
        notes_section
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn sample() -> Project {
        let mut p = Project::default();
        let id = p.alloc_id();
        p.items.push(PlacedItem {
            id,
            catalog_id: "sofa-3".into(),
            name: "3-seat sofa".into(),
            category: "Seating".into(),
            x_m: 2.5,
            y_m: 1.0,
            width_m: 2.2,
            depth_m: 0.95,
            height_m: 0.85,
            rotation_deg: 0.0,
            rgb: [96, 102, 116],
        });
        let oid = p.alloc_id();
        p.openings.push(Opening {
            id: oid,
            kind: OpeningKind::Door,
            wall: Wall::South,
            offset_m: 0.4,
            width_m: 0.9,
            sill_m: 0.0,
            height_m: 2.1,
            swings_in: true,
        });
        p
    }

    #[test]
    fn svg_contains_room_and_item() {
        let svg = plan_svg(&sample(), 100.0);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("3-seat sofa"));
        assert!(svg.contains("Floor area"));
    }

    #[test]
    fn scaled_svg_carries_physical_size() {
        let svg = plan_svg_scaled(&sample(), 50.0);
        // 5 m room at 100 px/m + 120 margin = 620 units → 124 mm at 1:50.
        assert!(svg.contains("width=\"124.0mm\""), "head: {}", &svg[..200]);
    }

    #[test]
    fn csv_has_header_and_grouped_row() {
        let csv = list_csv(&sample());
        assert!(csv.starts_with("Item,Category,"));
        assert!(csv.contains("3-seat sofa,Seating,2.20,0.95,0.85,1"));
        assert!(!csv.to_lowercase().contains("price"));
    }

    #[test]
    fn html_embeds_plan_and_list() {
        let html = print_html(&sample());
        assert!(html.contains("<svg"));
        assert!(html.contains("Item list"));
        assert!(html.contains("Door — South wall"));
    }
}
