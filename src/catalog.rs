//! The built-in furniture catalog. All items are original, generic pieces
//! with typical real-world dimensions (metres: width × depth × height).
//! There are deliberately no prices anywhere in RoomForge.

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogItem {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub width_m: f64,
    pub depth_m: f64,
    pub height_m: f64,
    pub rgb: [u8; 3],
}

macro_rules! item {
    ($id:expr, $name:expr, $cat:expr, $w:expr, $d:expr, $h:expr, $rgb:expr) => {
        CatalogItem {
            id: $id,
            name: $name,
            category: $cat,
            width_m: $w,
            depth_m: $d,
            height_m: $h,
            rgb: $rgb,
        }
    };
}

pub fn catalog() -> Vec<CatalogItem> {
    vec![
        // Seating
        item!(
            "sofa-3",
            "3-seat sofa",
            "Seating",
            2.20,
            0.95,
            0.85,
            [96, 102, 116]
        ),
        item!(
            "sofa-2",
            "2-seat sofa",
            "Seating",
            1.60,
            0.95,
            0.85,
            [110, 96, 88]
        ),
        item!(
            "armchair",
            "Armchair",
            "Seating",
            0.85,
            0.85,
            0.85,
            [128, 90, 74]
        ),
        item!(
            "dining-chair",
            "Dining chair",
            "Seating",
            0.45,
            0.50,
            0.90,
            [150, 118, 82]
        ),
        item!(
            "desk-chair",
            "Desk chair",
            "Seating",
            0.60,
            0.60,
            1.00,
            [52, 54, 60]
        ),
        item!(
            "stool",
            "Stool",
            "Seating",
            0.35,
            0.35,
            0.45,
            [170, 140, 100]
        ),
        item!(
            "bench",
            "Bench",
            "Seating",
            1.20,
            0.35,
            0.45,
            [140, 105, 70]
        ),
        item!(
            "beanbag",
            "Bean bag",
            "Seating",
            0.80,
            0.80,
            0.60,
            [160, 70, 60]
        ),
        // Tables
        item!(
            "dining-table-6",
            "Dining table (6)",
            "Tables",
            1.80,
            0.90,
            0.75,
            [146, 110, 72]
        ),
        item!(
            "dining-table-4",
            "Dining table (4)",
            "Tables",
            1.20,
            0.80,
            0.75,
            [146, 110, 72]
        ),
        item!(
            "coffee-table",
            "Coffee table",
            "Tables",
            1.10,
            0.55,
            0.40,
            [120, 88, 58]
        ),
        item!(
            "side-table",
            "Side table",
            "Tables",
            0.45,
            0.45,
            0.55,
            [120, 88, 58]
        ),
        item!("desk", "Desk", "Tables", 1.40, 0.65, 0.75, [104, 80, 56]),
        item!(
            "desk-small",
            "Desk, small",
            "Tables",
            1.00,
            0.55,
            0.75,
            [104, 80, 56]
        ),
        item!(
            "kitchen-island",
            "Kitchen island",
            "Tables",
            1.80,
            0.90,
            0.90,
            [90, 90, 96]
        ),
        // Beds
        item!(
            "bed-double",
            "Double bed",
            "Beds",
            1.60,
            2.10,
            0.90,
            [176, 168, 158]
        ),
        item!(
            "bed-single",
            "Single bed",
            "Beds",
            0.95,
            2.10,
            0.90,
            [176, 168, 158]
        ),
        item!(
            "bed-king",
            "King bed",
            "Beds",
            1.90,
            2.10,
            0.95,
            [168, 158, 148]
        ),
        item!(
            "cot",
            "Cot / crib",
            "Beds",
            1.40,
            0.75,
            0.90,
            [200, 196, 188]
        ),
        item!(
            "bedside-table",
            "Bedside table",
            "Beds",
            0.45,
            0.40,
            0.55,
            [120, 88, 58]
        ),
        // Storage
        item!(
            "wardrobe-2",
            "Wardrobe, 2-door",
            "Storage",
            1.20,
            0.60,
            2.00,
            [132, 104, 76]
        ),
        item!(
            "wardrobe-3",
            "Wardrobe, 3-door",
            "Storage",
            1.80,
            0.60,
            2.00,
            [132, 104, 76]
        ),
        item!(
            "chest-4",
            "Chest of drawers",
            "Storage",
            0.90,
            0.50,
            1.05,
            [140, 110, 80]
        ),
        item!(
            "bookcase",
            "Bookcase",
            "Storage",
            0.90,
            0.30,
            1.80,
            [126, 96, 64]
        ),
        item!(
            "shelf-unit",
            "Shelf unit (4×4)",
            "Storage",
            1.47,
            0.39,
            1.47,
            [110, 84, 60]
        ),
        item!(
            "tv-bench",
            "TV bench",
            "Storage",
            1.60,
            0.42,
            0.45,
            [96, 74, 52]
        ),
        item!(
            "sideboard",
            "Sideboard",
            "Storage",
            1.60,
            0.45,
            0.70,
            [130, 100, 70]
        ),
        item!(
            "shoe-cabinet",
            "Shoe cabinet",
            "Storage",
            0.90,
            0.35,
            1.00,
            [140, 112, 84]
        ),
        item!(
            "storage-box",
            "Storage box",
            "Storage",
            0.60,
            0.40,
            0.35,
            [150, 150, 154]
        ),
        // Kitchen
        item!(
            "base-cabinet-60",
            "Base cabinet 60",
            "Kitchen",
            0.60,
            0.60,
            0.90,
            [188, 188, 190]
        ),
        item!(
            "sink-unit",
            "Sink unit",
            "Kitchen",
            0.80,
            0.60,
            0.90,
            [178, 182, 186]
        ),
        item!(
            "cooker",
            "Cooker / oven",
            "Kitchen",
            0.60,
            0.60,
            0.90,
            [70, 70, 76]
        ),
        item!(
            "fridge",
            "Fridge-freezer",
            "Kitchen",
            0.70,
            0.65,
            1.85,
            [200, 202, 206]
        ),
        item!(
            "dishwasher",
            "Dishwasher",
            "Kitchen",
            0.60,
            0.60,
            0.85,
            [190, 192, 196]
        ),
        item!(
            "washer",
            "Washing machine",
            "Kitchen",
            0.60,
            0.60,
            0.85,
            [196, 198, 202]
        ),
        // Bathroom
        item!(
            "bath",
            "Bath",
            "Bathroom",
            1.70,
            0.75,
            0.60,
            [232, 234, 236]
        ),
        item!(
            "shower",
            "Shower cubicle",
            "Bathroom",
            0.90,
            0.90,
            2.00,
            [190, 205, 215]
        ),
        item!(
            "wc",
            "Toilet",
            "Bathroom",
            0.40,
            0.65,
            0.80,
            [236, 238, 240]
        ),
        item!(
            "basin-unit",
            "Basin unit",
            "Bathroom",
            0.80,
            0.48,
            0.85,
            [210, 206, 198]
        ),
        // Lighting, decor and other
        item!(
            "floor-lamp",
            "Floor lamp",
            "Lighting & decor",
            0.35,
            0.35,
            1.60,
            [220, 190, 120]
        ),
        item!(
            "rug-large",
            "Rug, large",
            "Lighting & decor",
            2.00,
            1.40,
            0.02,
            [150, 60, 55]
        ),
        item!(
            "rug-small",
            "Rug, small",
            "Lighting & decor",
            1.20,
            0.80,
            0.02,
            [90, 110, 130]
        ),
        item!(
            "plant",
            "Plant",
            "Lighting & decor",
            0.40,
            0.40,
            0.90,
            [70, 120, 70]
        ),
        item!(
            "tv-55",
            "TV (on stand)",
            "Lighting & decor",
            1.25,
            0.30,
            0.80,
            [30, 30, 34]
        ),
        item!(
            "piano",
            "Upright piano",
            "Other",
            1.50,
            0.60,
            1.30,
            [40, 36, 34]
        ),
        item!(
            "exercise-bike",
            "Exercise bike",
            "Other",
            1.10,
            0.55,
            1.40,
            [80, 84, 92]
        ),
        item!(
            "cot-change",
            "Changing table",
            "Other",
            0.85,
            0.75,
            0.95,
            [200, 192, 180]
        ),
        item!(
            "dog-bed",
            "Dog bed",
            "Other",
            0.90,
            0.65,
            0.25,
            [140, 100, 80]
        ),
        item!(
            "custom",
            "Custom item",
            "Other",
            0.60,
            0.60,
            0.80,
            [120, 120, 128]
        ),
    ]
}

pub fn categories() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for it in catalog() {
        if !out.contains(&it.category) {
            out.push(it.category);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_no_duplicate_ids_and_sane_dimensions() {
        let items = catalog();
        assert!(items.len() >= 40, "catalog too small: {}", items.len());
        for (i, a) in items.iter().enumerate() {
            assert!(
                a.width_m > 0.0 && a.depth_m > 0.0 && a.height_m > 0.0,
                "{}",
                a.id
            );
            for b in items.iter().skip(i + 1) {
                assert_ne!(a.id, b.id);
            }
        }
    }

    #[test]
    fn every_catalog_item_belongs_to_a_listed_category() {
        let cats = categories();
        for it in catalog() {
            assert!(cats.contains(&it.category));
        }
    }
}
