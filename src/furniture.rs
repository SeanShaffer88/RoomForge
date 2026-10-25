//! The furniture designer: parametric pieces built from panels.
//!
//! A design is a template (closet, chest of drawers…) plus overall
//! dimensions, a finish and a couple of options. Its parts — sides,
//! shelves, doors, drawer boxes — are computed deterministically
//! from those parameters and never stored, so a design can never
//! disagree with its own parts list. The cut list carries
//! dimensions and quantities only: no prices, anywhere, ever.

use serde::{Deserialize, Serialize};

use crate::view3d::{box_faces, FaceQuad};

/// Panel thickness for carcasses, doors and shelves (18 mm).
pub const PANEL_T: f64 = 0.018;
const BACK_T: f64 = 0.006;
const PLINTH_H: f64 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Template {
    Closet,
    DoubleCloset,
    Chest,
    SideTable,
    Bookcase,
}

impl Template {
    pub fn label(self) -> &'static str {
        match self {
            Template::Closet => "Closet",
            Template::DoubleCloset => "Double closet",
            Template::Chest => "Chest of drawers",
            Template::SideTable => "Side table",
            Template::Bookcase => "Bookcase",
        }
    }
    /// The catalog category a placed piece belongs to.
    pub fn category(self) -> &'static str {
        match self {
            Template::SideTable => "Tables",
            _ => "Storage",
        }
    }
    /// (width, depth, height) a new design starts with, metres.
    pub fn default_dims(self) -> (f64, f64, f64) {
        match self {
            Template::Closet => (1.2, 0.6, 2.1),
            Template::DoubleCloset => (2.0, 0.6, 2.1),
            Template::Chest => (0.9, 0.5, 1.1),
            Template::SideTable => (0.5, 0.4, 0.55),
            Template::Bookcase => (0.9, 0.32, 1.8),
        }
    }
    pub fn uses_drawers(self) -> bool {
        matches!(
            self,
            Template::Closet | Template::DoubleCloset | Template::Chest
        )
    }
    pub fn uses_shelves(self) -> bool {
        matches!(
            self,
            Template::Closet | Template::DoubleCloset | Template::Bookcase
        )
    }
    pub fn drawer_range(self) -> (u32, u32) {
        match self {
            Template::Chest => (2, 6),
            Template::Closet | Template::DoubleCloset => (0, 3),
            _ => (0, 0),
        }
    }
    pub fn shelf_range(self) -> (u32, u32) {
        match self {
            Template::Bookcase => (0, 6),
            Template::Closet | Template::DoubleCloset => (0, 4),
            _ => (0, 0),
        }
    }
    pub fn all() -> [Template; 5] {
        [
            Template::Closet,
            Template::DoubleCloset,
            Template::Chest,
            Template::SideTable,
            Template::Bookcase,
        ]
    }
}

/// One designed piece of furniture, stored in the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FurnitureDesign {
    pub id: u64,
    pub name: String,
    pub template: Template,
    pub width_m: f64,
    pub depth_m: f64,
    pub height_m: f64,
    pub finish_rgb: [u8; 3],
    #[serde(default)]
    pub drawers: u32,
    #[serde(default)]
    pub shelves: u32,
}

/// The standard finishes offered in the designer.
pub fn finishes() -> [(&'static str, [u8; 3]); 6] {
    [
        ("Oak", [176, 141, 102]),
        ("Walnut", [110, 79, 58]),
        ("White", [232, 230, 226]),
        ("Matt black", [38, 36, 38]),
        ("Slate grey", [112, 116, 122]),
        ("Sage", [138, 154, 128]),
    ]
}

impl FurnitureDesign {
    pub fn new(id: u64, template: Template) -> Self {
        let (w, d, h) = template.default_dims();
        FurnitureDesign {
            id,
            name: template.label().into(),
            template,
            width_m: w,
            depth_m: d,
            height_m: h,
            finish_rgb: finishes()[0].1,
            drawers: match template {
                Template::Chest => 4,
                _ => 0,
            },
            shelves: match template {
                Template::Bookcase => 3,
                Template::Closet => 1,
                _ => 0,
            },
        }
    }

    pub fn catalog_id(&self) -> String {
        format!("furn-{}", self.id)
    }

    /// Clamp dimensions and options to what the template supports,
    /// so a design is always buildable.
    pub fn clamped(mut self) -> Self {
        self.width_m = self.width_m.clamp(0.3, 3.0);
        self.depth_m = self.depth_m.clamp(0.24, 0.9);
        self.height_m = self.height_m.clamp(0.3, 2.6);
        let (dmin, dmax) = self.template.drawer_range();
        self.drawers = self.drawers.clamp(dmin, dmax);
        let (smin, smax) = self.template.shelf_range();
        self.shelves = self.shelves.clamp(smin, smax);
        self
    }

    /// Every panel and part of the piece, in piece space: origin at
    /// the floor centre, x across the width, y across the depth
    /// (front faces +y), z up.
    pub fn parts(&self) -> Vec<Part> {
        let d = self.clone().clamped();
        let mut b = Builder {
            base: d.finish_rgb,
            parts: Vec::new(),
        };
        let int = carcass(&d, &mut b);
        match d.template {
            Template::Closet => {
                doors(&d, &mut b, int, int.x0, int.x1);
                shelves(&d, &mut b, int, int.x0, int.x1);
                drawers(&d, &mut b, int, int.x0, int.x1, d.drawers);
            }
            Template::DoubleCloset => {
                // A centre divider makes two bays: shelves on the
                // right, drawers low on the left, two doors each.
                b.push(Part {
                    name: "Divider".into(),
                    role: Role::Divider,
                    cx: 0.0,
                    cy: 0.0,
                    cz: (int.z0 + int.z1) / 2.0,
                    w: PANEL_T,
                    d: d.depth_m - 0.03,
                    h: int.z1 - int.z0,
                    hinge: None,
                    group: 0,
                    rgb: shade(d.finish_rgb, 1.0),
                });
                let mid = PANEL_T / 2.0;
                doors(&d, &mut b, int, int.x0, -mid);
                doors(&d, &mut b, int, mid, int.x1);
                shelves(&d, &mut b, int, mid, int.x1);
                drawers(&d, &mut b, int, int.x0, -mid, d.drawers);
            }
            Template::Chest => {
                drawers(&d, &mut b, int, int.x0, int.x1, d.drawers.max(1));
            }
            Template::SideTable => {
                // One drawer at the top, a door below it.
                let front_h = (int.z1 - int.z0) * 0.32;
                drawer_unit(&d, &mut b, int.x0, int.x1, int.z1 - front_h, int.z1, 1);
                let dz0 = int.z0;
                let dz1 = int.z1 - front_h - 0.02;
                if dz1 > dz0 + 0.1 {
                    b.push(Part {
                        name: "Door".into(),
                        role: Role::Door,
                        cx: (int.x0 + int.x1) / 2.0,
                        cy: d.depth_m / 2.0 - 0.009,
                        cz: (dz0 + dz1) / 2.0,
                        w: (int.x1 - int.x0) - 0.004,
                        d: PANEL_T,
                        h: dz1 - dz0,
                        hinge: Some(Hinge::Right),
                        group: 0,
                        rgb: shade(d.finish_rgb, 0.92),
                    });
                }
            }
            Template::Bookcase => {
                shelves(&d, &mut b, int, int.x0, int.x1);
            }
        }
        b.parts
    }

    /// The cut list: identical parts grouped, dimensions in metres.
    pub fn cut_list(&self) -> Vec<CutEntry> {
        let mut out: Vec<CutEntry> = Vec::new();
        for p in self.parts() {
            // A part's sheet dimensions: its two largest extents are
            // the panel's length/width; the smallest is thickness.
            let mut dims = [p.w, p.d, p.h];
            dims.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let (thickness, width, length) = (dims[0], dims[1], dims[2]);
            let key = (
                p.name.clone(),
                (length * 1000.0).round() as i64,
                (width * 1000.0).round() as i64,
                (thickness * 1000.0).round() as i64,
            );
            if let Some(e) = out.iter_mut().find(|e: &&mut CutEntry| {
                e.name == key.0
                    && (e.length_m * 1000.0).round() as i64 == key.1
                    && (e.width_m * 1000.0).round() as i64 == key.2
                    && (e.thickness_m * 1000.0).round() as i64 == key.3
            }) {
                e.qty += 1;
            } else {
                out.push(CutEntry {
                    name: p.name.clone(),
                    qty: 1,
                    length_m: length,
                    width_m: width,
                    thickness_m: thickness,
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Total panel area of the piece, square metres.
    pub fn board_area_m2(&self) -> f64 {
        self.cut_list()
            .iter()
            .map(|e| e.length_m * e.width_m * e.qty as f64)
            .sum()
    }

    /// A plain-text CSV of the cut list (dimensions + quantities).
    pub fn cut_list_csv(&self) -> String {
        let mut s = String::from("part,quantity,length_m,width_m,thickness_m\n");
        for e in self.cut_list() {
            s.push_str(&format!(
                "{},{},{:.3},{:.3},{:.3}\n",
                e.name, e.qty, e.length_m, e.width_m, e.thickness_m
            ));
        }
        s
    }
}

/// The design id inside a `furn-<id>` catalog id, if it is one.
pub fn furn_id_of(catalog_id: &str) -> Option<u64> {
    catalog_id.strip_prefix("furn-")?.parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Side,
    Top,
    Bottom,
    Back,
    Plinth,
    Shelf,
    Divider,
    Door,
    DrawerFront,
    DrawerBox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hinge {
    Left,
    Right,
}

/// One panel or part of a design, in piece space.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    pub role: Role,
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub w: f64,
    pub d: f64,
    pub h: f64,
    pub hinge: Option<Hinge>,
    /// Drawer fronts and their boxes share a group (1-based);
    /// 0 = ungrouped.
    pub group: u32,
    pub rgb: [u8; 3],
}

/// One line of a cut list.
#[derive(Debug, Clone, PartialEq)]
pub struct CutEntry {
    pub name: String,
    pub qty: u32,
    pub length_m: f64,
    pub width_m: f64,
    pub thickness_m: f64,
}

fn shade(rgb: [u8; 3], f: f64) -> [u8; 3] {
    std::array::from_fn(|i| ((rgb[i] as f64 * f).clamp(0.0, 255.0)) as u8)
}

struct Builder {
    base: [u8; 3],
    parts: Vec<Part>,
}

impl Builder {
    fn push(&mut self, p: Part) {
        self.parts.push(p);
    }
}

/// The interior bounds of a piece once its carcass exists.
#[derive(Clone, Copy)]
struct Interior {
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
}

/// Build the carcass shared by every template: two sides, top,
/// bottom, back and (optionally) a plinth.
fn carcass(d: &FurnitureDesign, b: &mut Builder) -> Interior {
    let (w, dep, h) = (d.width_m, d.depth_m, d.height_m);
    let plinth = match d.template {
        Template::SideTable => 0.0,
        _ => PLINTH_H,
    };
    for (name, cx) in [
        ("Side", -(w / 2.0 - PANEL_T / 2.0)),
        ("Side", w / 2.0 - PANEL_T / 2.0),
    ] {
        b.push(Part {
            name: name.into(),
            role: Role::Side,
            cx,
            cy: 0.0,
            cz: h / 2.0,
            w: PANEL_T,
            d: dep,
            h,
            hinge: None,
            group: 0,
            rgb: shade(b.base, 1.0),
        });
    }
    b.push(Part {
        name: "Top".into(),
        role: Role::Top,
        cx: 0.0,
        cy: 0.0,
        cz: h - PANEL_T / 2.0,
        w: w - 2.0 * PANEL_T,
        d: dep,
        h: PANEL_T,
        hinge: None,
        group: 0,
        rgb: shade(b.base, 1.04),
    });
    b.push(Part {
        name: "Bottom".into(),
        role: Role::Bottom,
        cx: 0.0,
        cy: 0.0,
        cz: plinth + PANEL_T / 2.0,
        w: w - 2.0 * PANEL_T,
        d: dep,
        h: PANEL_T,
        hinge: None,
        group: 0,
        rgb: shade(b.base, 0.96),
    });
    if plinth > 0.0 {
        b.push(Part {
            name: "Plinth".into(),
            role: Role::Plinth,
            cx: 0.0,
            cy: dep / 2.0 - 0.03,
            cz: plinth / 2.0,
            w: w - 2.0 * PANEL_T,
            d: 0.02,
            h: plinth,
            hinge: None,
            group: 0,
            rgb: shade(b.base, 0.8),
        });
    }
    let z0 = plinth + PANEL_T;
    let z1 = h - PANEL_T;
    b.push(Part {
        name: "Back".into(),
        role: Role::Back,
        cx: 0.0,
        cy: -dep / 2.0 + BACK_T / 2.0 + 0.004,
        cz: (z0 + z1) / 2.0,
        w: w - 2.0 * PANEL_T,
        d: BACK_T,
        h: z1 - z0,
        hinge: None,
        group: 0,
        rgb: shade(b.base, 1.12),
    });
    Interior {
        x0: -(w / 2.0 - PANEL_T),
        x1: w / 2.0 - PANEL_T,
        z0,
        z1,
    }
}

/// A pair of doors across [x0, x1] of the interior front.
fn doors(d: &FurnitureDesign, b: &mut Builder, int: Interior, x0: f64, x1: f64) {
    let mid = (x0 + x1) / 2.0;
    let h = int.z1 - int.z0 - 0.004;
    for (cx0, cx1, hinge) in [(x0, mid, Hinge::Left), (mid, x1, Hinge::Right)] {
        b.push(Part {
            name: "Door".into(),
            role: Role::Door,
            cx: (cx0 + cx1) / 2.0,
            cy: d.depth_m / 2.0 - 0.009,
            cz: (int.z0 + int.z1) / 2.0,
            w: (cx1 - cx0) - 0.003,
            d: PANEL_T,
            h,
            hinge: Some(hinge),
            group: 0,
            rgb: shade(b.base, 0.92),
        });
    }
}

/// The design's shelves, evenly spaced through the interior.
fn shelves(d: &FurnitureDesign, b: &mut Builder, int: Interior, x0: f64, x1: f64) {
    let n = d.shelves;
    for i in 0..n {
        let z = int.z0 + (int.z1 - int.z0) * (i + 1) as f64 / (n + 1) as f64;
        b.push(Part {
            name: "Shelf".into(),
            role: Role::Shelf,
            cx: (x0 + x1) / 2.0,
            cy: -0.01,
            cz: z,
            w: (x1 - x0) - 0.004,
            d: d.depth_m - 0.05,
            h: PANEL_T,
            hinge: None,
            group: 0,
            rgb: shade(b.base, 1.06),
        });
    }
}

/// A stack of drawers in the bottom of [x0, x1] (or the whole
/// interior for a chest).
fn drawers(d: &FurnitureDesign, b: &mut Builder, int: Interior, x0: f64, x1: f64, count: u32) {
    if count == 0 {
        return;
    }
    let zone = match d.template {
        Template::Chest => (int.z0, int.z1),
        _ => (int.z0, int.z0 + (int.z1 - int.z0) * 0.45),
    };
    let each = (zone.1 - zone.0) / count as f64;
    for i in 0..count {
        let z0 = zone.0 + each * i as f64;
        drawer_unit(d, b, x0, x1, z0, z0 + each, i + 1);
    }
}

/// One drawer: a front on the face plus a box behind it.
fn drawer_unit(
    d: &FurnitureDesign,
    b: &mut Builder,
    x0: f64,
    x1: f64,
    z0: f64,
    z1: f64,
    group: u32,
) {
    b.push(Part {
        name: "Drawer front".into(),
        role: Role::DrawerFront,
        cx: (x0 + x1) / 2.0,
        cy: d.depth_m / 2.0 - 0.009,
        cz: (z0 + z1) / 2.0,
        w: (x1 - x0) - 0.004,
        d: PANEL_T,
        h: (z1 - z0) - 0.004,
        hinge: None,
        group,
        rgb: shade(b.base, 0.92),
    });
    b.push(Part {
        name: "Drawer box".into(),
        role: Role::DrawerBox,
        cx: (x0 + x1) / 2.0,
        cy: -0.03,
        cz: (z0 + z1) / 2.0,
        w: (x1 - x0) - 0.09,
        d: d.depth_m - 0.12,
        h: ((z1 - z0) - 0.05).max(0.03),
        hinge: None,
        group,
        rgb: [206, 196, 180],
    });
}

/// The design's faces in piece space: every part as a shaded box,
/// doors swung and drawers slid by `open` (0..1), parts drifted
/// apart by `explode` (0..1). Renderers place these onto a room
/// item (see view3d::place_faces) or a preview camera directly.
pub fn design_faces(design: &FurnitureDesign, open: f64, explode: f64) -> Vec<FaceQuad> {
    let open = open.clamp(0.0, 1.0);
    let mut out = Vec::new();
    for part in design.parts() {
        // Exploded view: drift away from the piece's centre axis.
        let (mut cx, mut cy) = (part.cx, part.cy);
        if explode > 0.0 {
            let dist = (cx * cx + cy * cy).sqrt();
            if dist > 0.02 {
                let shift = explode * 0.2 * dist.min(0.8);
                cx += cx / dist * shift;
                cy += cy / dist * shift;
            }
        }
        // The part's 8 corners in the standard box order.
        let (hw, hd, hh) = (part.w / 2.0, part.d / 2.0, part.h / 2.0);
        let plan = [(-hw, -hd), (hw, -hd), (hw, hd), (-hw, hd)];
        let mut corners = [(0.0, 0.0, 0.0); 8];
        for (i, &(lx, ly)) in plan.iter().enumerate() {
            corners[i] = (cx + lx, cy + ly, part.cz - hh);
            corners[i + 4] = (cx + lx, cy + ly, part.cz + hh);
        }
        // Opening motion.
        match part.role {
            Role::Door => {
                if open > 0.0 {
                    if let Some(hinge) = part.hinge {
                        let hx = cx + if hinge == Hinge::Left { -hw } else { hw };
                        let a = if hinge == Hinge::Left {
                            open * 100.0f64.to_radians()
                        } else {
                            -open * 100.0f64.to_radians()
                        };
                        for c in corners.iter_mut() {
                            let (rx, ry) = (c.0 - hx, c.1 - cy);
                            c.0 = hx + rx * a.cos() - ry * a.sin();
                            c.1 = cy + rx * a.sin() + ry * a.cos();
                        }
                    }
                }
            }
            Role::DrawerFront | Role::DrawerBox if open > 0.0 && part.group > 0 => {
                let slide = open * (design.depth_m * 0.65).min(0.32);
                for c in corners.iter_mut() {
                    c.1 += slide;
                }
            }
            _ => {}
        }
        for (idx, bright) in box_faces() {
            out.push((
                [
                    corners[idx[0]],
                    corners[idx[1]],
                    corners[idx[2]],
                    corners[idx[3]],
                ],
                bright,
                part.rgb,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn design(t: Template) -> FurnitureDesign {
        FurnitureDesign::new(1, t).clamped()
    }

    #[test]
    fn closet_parts_stay_inside_the_piece() {
        let d = design(Template::Closet);
        let parts = d.parts();
        assert!(parts.len() >= 9, "carcass + doors + shelf: {}", parts.len());
        for p in &parts {
            assert!(p.cx - p.w / 2.0 >= -d.width_m / 2.0 - 1e-9, "{}", p.name);
            assert!(p.cx + p.w / 2.0 <= d.width_m / 2.0 + 1e-9, "{}", p.name);
            assert!(p.cz - p.h / 2.0 >= -1e-9, "{} sinks", p.name);
            assert!(
                p.cz + p.h / 2.0 <= d.height_m + 1e-9,
                "{} pokes out",
                p.name
            );
        }
        assert_eq!(
            parts.iter().filter(|p| p.role == Role::Door).count(),
            2,
            "a closet has two doors"
        );
    }

    #[test]
    fn chest_drawers_and_cut_list() {
        let mut d = design(Template::Chest);
        d.drawers = 4;
        let parts = d.parts();
        assert_eq!(
            parts.iter().filter(|p| p.role == Role::DrawerFront).count(),
            4
        );
        assert_eq!(
            parts.iter().filter(|p| p.role == Role::DrawerBox).count(),
            4
        );
        let cut = d.cut_list();
        let fronts = cut.iter().find(|e| e.name == "Drawer front").unwrap();
        assert_eq!(fronts.qty, 4, "identical fronts group together");
        assert!(d.board_area_m2() > 1.0, "a chest needs real board");
        let csv = d.cut_list_csv();
        assert!(csv.starts_with("part,quantity,"));
        assert!(csv.contains("Drawer front,4,"));
    }

    #[test]
    fn double_closet_has_four_doors_and_a_divider() {
        let d = design(Template::DoubleCloset);
        let parts = d.parts();
        assert_eq!(parts.iter().filter(|p| p.role == Role::Door).count(), 4);
        assert_eq!(parts.iter().filter(|p| p.role == Role::Divider).count(), 1);
    }

    #[test]
    fn faces_open_and_explode() {
        let d = design(Template::Closet);
        let closed = design_faces(&d, 0.0, 0.0);
        let open = design_faces(&d, 1.0, 0.0);
        assert_eq!(closed.len(), open.len());
        assert_ne!(closed, open, "opening the doors changes the faces");
        let boom = design_faces(&d, 0.0, 1.0);
        assert_ne!(closed, boom, "exploding changes the faces");
        // Closed doors stay within the footprint (plus a hair).
        for (quad, _, _) in &closed {
            for pt in quad {
                assert!(pt.0.abs() <= d.width_m / 2.0 + 0.02, "{pt:?}");
                assert!(pt.1.abs() <= d.depth_m / 2.0 + 0.02, "{pt:?}");
            }
        }
    }

    #[test]
    fn clamping_keeps_designs_buildable() {
        let mut d = FurnitureDesign::new(7, Template::Chest);
        d.width_m = 99.0;
        d.drawers = 42;
        let d = d.clamped();
        assert_eq!(d.width_m, 3.0);
        assert_eq!(d.drawers, 6);
        assert_eq!(d.catalog_id(), "furn-7");
        assert_eq!(furn_id_of("furn-7"), Some(7));
        assert_eq!(furn_id_of("sofa-3"), None);
    }
}
