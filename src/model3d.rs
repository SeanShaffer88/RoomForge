//! Imported 3D models: minimal GLB (binary glTF) and OBJ parsers.
//!
//! RoomForge renders furniture as flat-shaded boxes; a user-imported
//! model gives a placed item real geometry in the 3D view, the
//! walkthrough and snapshots. The parsers here are deliberately
//! small: static meshes only, no skins, animation, textures or
//! PBR beyond a base colour per primitive. That covers the models
//! people actually export for furniture (Blender, Sweet Home 3D,
//! asset stores) while keeping the dependency list empty.
//!
//! Parsed models are *prepared* into a unit space: centred on the
//! origin in X/Y, sitting on Z = 0, width exactly 1 — depth and
//! height come back as ratios, so the renderer can scale the mesh
//! onto a placed item's footprint with plain multiplication.

use crate::model::CustomModelDef;

/// One flat-shaded part of a model: triangle soup in prepared unit
/// space (see the module docs), plus its base colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub tris: Vec<[[f32; 3]; 3]>,
    pub rgb: [u8; 3],
}

/// A parsed, prepared model ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedModel {
    pub meshes: Vec<Mesh>,
    /// Depth relative to a width of 1.
    pub unit_depth: f64,
    /// Height relative to a width of 1.
    pub unit_height: f64,
    /// The model's width in its own units (metres for glTF; OBJ
    /// files carry no units, so metres are assumed).
    pub natural_width: f64,
}

const MAX_TRIS: usize = 500_000;
const DEFAULT_RGB: [u8; 3] = [168, 162, 154];

/// Parse + prepare a model held by a plan's model definition.
pub fn prepare(def: &CustomModelDef) -> Result<PreparedModel, String> {
    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        def.data_base64.as_bytes(),
    )
    .map_err(|e| format!("The stored model data is damaged: {e}"))?;
    prepare_bytes(&bytes, &def.format)
}

/// Parse + prepare raw model file bytes ("glb" or "obj").
pub fn prepare_bytes(bytes: &[u8], format: &str) -> Result<PreparedModel, String> {
    let mut meshes = match format {
        "glb" => parse_glb(bytes)?,
        "obj" => parse_obj(bytes)?,
        other => return Err(format!("Unsupported model format: {other}")),
    };
    if meshes.iter().map(|m| m.tris.len()).sum::<usize>() > MAX_TRIS {
        return Err("That model is very detailed (over 500k triangles) — RoomForge keeps models furniture-sized; try a decimated/lower-poly export.".into());
    }
    // World-frame conversion (Y-up source → Z-up room) happens in
    // the parsers; here we centre, ground and scale to width 1.
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for m in &meshes {
        for t in &m.tris {
            for v in t {
                for a in 0..3 {
                    min[a] = min[a].min(v[a]);
                    max[a] = max[a].max(v[a]);
                }
            }
        }
    }
    if min[0] == f32::MAX {
        return Err("That model contains no geometry.".into());
    }
    let w = (max[0] - min[0]) as f64;
    let d = (max[1] - min[1]) as f64;
    let h = (max[2] - min[2]) as f64;
    if w < 1e-6 {
        return Err("That model's width is zero — it looks like a flat plane.".into());
    }
    let cx = (min[0] + max[0]) / 2.0;
    let cy = (min[1] + max[1]) / 2.0;
    let s = (1.0 / w) as f32;
    for m in &mut meshes {
        for t in &mut m.tris {
            for v in t.iter_mut() {
                *v = [(v[0] - cx) * s, (v[1] - cy) * s, (v[2] - min[2]) * s];
            }
        }
    }
    Ok(PreparedModel {
        meshes,
        unit_depth: d / w,
        unit_height: h / w,
        natural_width: w,
    })
}

// ---------------------------------------------------------------- OBJ

fn parse_obj(bytes: &[u8]) -> Result<Vec<Mesh>, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("v ") {
            let mut it = rest.split_whitespace();
            let (Some(x), Some(y), Some(z)) = (it.next(), it.next(), it.next()) else {
                continue;
            };
            let (Ok(x), Ok(y), Ok(z)) = (x.parse(), y.parse(), z.parse()) else {
                continue;
            };
            // Y-up → Z-up room frame: (x, z, y).
            verts.push([x, z, y]);
        } else if let Some(rest) = line.strip_prefix("f ") {
            let idx: Vec<usize> = rest
                .split_whitespace()
                .filter_map(|tok| {
                    let raw: i64 = tok.split('/').next()?.parse().ok()?;
                    let i = if raw < 0 {
                        verts.len() as i64 + raw
                    } else {
                        raw - 1
                    };
                    if i >= 0 && (i as usize) < verts.len() {
                        Some(i as usize)
                    } else {
                        None
                    }
                })
                .collect();
            if idx.len() >= 3 {
                // Fan triangulation for n-gons.
                for k in 1..idx.len() - 1 {
                    tris.push([verts[idx[0]], verts[idx[k]], verts[idx[k + 1]]]);
                }
            }
        }
    }
    if tris.is_empty() {
        return Err("No faces found in that OBJ file.".into());
    }
    Ok(vec![Mesh {
        tris,
        rgb: DEFAULT_RGB,
    }])
}

// ---------------------------------------------------------------- GLB

type Mat4 = [f32; 16];

fn mat_identity() -> Mat4 {
    let mut m = [0.0; 16];
    m[0] = 1.0;
    m[5] = 1.0;
    m[10] = 1.0;
    m[15] = 1.0;
    m
}

fn mat_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    out
}

fn mat_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn mat_from_trs(t: [f32; 3], q: [f32; 4], s: [f32; 3]) -> Mat4 {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    // Column-major rotation, then scale columns, then translate.
    let r: Mat4 = [
        (1.0 - 2.0 * (yy + zz)) * s[0],
        2.0 * (xy + wz) * s[0],
        2.0 * (xz - wy) * s[0],
        0.0,
        2.0 * (xy - wz) * s[1],
        (1.0 - 2.0 * (xx + zz)) * s[1],
        2.0 * (yz + wx) * s[1],
        0.0,
        2.0 * (xz + wy) * s[2],
        2.0 * (yz - wx) * s[2],
        (1.0 - 2.0 * (xx + yy)) * s[2],
        0.0,
        t[0],
        t[1],
        t[2],
        1.0,
    ];
    r
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

struct Gltf<'a> {
    json: serde_json::Value,
    bin: &'a [u8],
}

impl<'a> Gltf<'a> {
    fn accessor_f32x3(&self, idx: usize) -> Result<Vec<[f32; 3]>, String> {
        let acc = &self.json["accessors"][idx];
        if acc["componentType"].as_u64() != Some(5126) || acc["type"].as_str() != Some("VEC3") {
            return Err("A model mesh uses an unsupported vertex format.".into());
        }
        let count = acc["count"].as_u64().unwrap_or(0) as usize;
        let bv = &self.json["bufferViews"][acc["bufferView"].as_u64().unwrap_or(0) as usize];
        let base = bv["byteOffset"].as_u64().unwrap_or(0) as usize
            + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
        let stride = bv["byteStride"].as_u64().unwrap_or(12) as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let at = base + i * stride.max(12);
            let mut v = [0.0f32; 3];
            for (a, slot) in v.iter_mut().enumerate() {
                let b = self
                    .bin
                    .get(at + a * 4..at + a * 4 + 4)
                    .ok_or("Model data runs past the end of the file.")?;
                *slot = f32::from_le_bytes(b.try_into().unwrap());
            }
            // Y-up → Z-up room frame: (x, z, y).
            out.push([v[0], v[2], v[1]]);
        }
        Ok(out)
    }

    fn accessor_indices(&self, idx: usize) -> Result<Vec<u32>, String> {
        let acc = &self.json["accessors"][idx];
        let count = acc["count"].as_u64().unwrap_or(0) as usize;
        let bv = &self.json["bufferViews"][acc["bufferView"].as_u64().unwrap_or(0) as usize];
        let base = bv["byteOffset"].as_u64().unwrap_or(0) as usize
            + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
        let mut out = Vec::with_capacity(count);
        match acc["componentType"].as_u64() {
            Some(5121) => {
                for i in 0..count {
                    out.push(
                        *self
                            .bin
                            .get(base + i)
                            .ok_or("Model indices run past the end.")?
                            as u32,
                    );
                }
            }
            Some(5123) => {
                for i in 0..count {
                    let b = self
                        .bin
                        .get(base + i * 2..base + i * 2 + 2)
                        .ok_or("Model indices run past the end.")?;
                    out.push(u16::from_le_bytes(b.try_into().unwrap()) as u32);
                }
            }
            Some(5125) => {
                for i in 0..count {
                    out.push(
                        u32le(self.bin, base + i * 4).ok_or("Model indices run past the end.")?,
                    );
                }
            }
            _ => return Err("A model mesh uses an unsupported index format.".into()),
        }
        Ok(out)
    }

    fn walk_node(&self, node_idx: usize, parent: &Mat4, out: &mut Vec<Mesh>) -> Result<(), String> {
        let node = &self.json["nodes"][node_idx];
        let local = if let Some(m) = node["matrix"].as_array() {
            let mut mat = [0.0f32; 16];
            for (i, v) in m.iter().enumerate().take(16) {
                mat[i] = v.as_f64().unwrap_or(0.0) as f32;
            }
            mat
        } else {
            let t = node["translation"].as_array();
            let r = node["rotation"].as_array();
            let s = node["scale"].as_array();
            let grab = |a: Option<&Vec<serde_json::Value>>, i: usize, d: f32| {
                a.and_then(|v| v.get(i))
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(d)
            };
            mat_from_trs(
                [grab(t, 0, 0.0), grab(t, 1, 0.0), grab(t, 2, 0.0)],
                [
                    grab(r, 0, 0.0),
                    grab(r, 1, 0.0),
                    grab(r, 2, 0.0),
                    grab(r, 3, 1.0),
                ],
                [grab(s, 0, 1.0), grab(s, 1, 1.0), grab(s, 2, 1.0)],
            )
        };
        let world = mat_mul(parent, &local);
        if let Some(mesh_idx) = node["mesh"].as_u64() {
            let mesh = &self.json["meshes"][mesh_idx as usize];
            if let Some(prims) = mesh["primitives"].as_array() {
                for prim in prims {
                    let pos_idx = prim["attributes"]["POSITION"]
                        .as_u64()
                        .ok_or("A model mesh has no positions.")?
                        as usize;
                    let verts = self.accessor_f32x3(pos_idx)?;
                    let rgb = prim["material"]
                        .as_u64()
                        .and_then(|mi| {
                            let f = &self.json["materials"][mi as usize]["pbrMetallicRoughness"]
                                ["baseColorFactor"];
                            f.as_array().map(|a| {
                                let c = |i: usize| {
                                    (a.get(i).and_then(|v| v.as_f64()).unwrap_or(1.0) * 255.0)
                                        .clamp(0.0, 255.0) as u8
                                };
                                [c(0), c(1), c(2)]
                            })
                        })
                        .unwrap_or(DEFAULT_RGB);
                    let mut tris = Vec::new();
                    if let Some(ii) = prim["indices"].as_u64() {
                        let idx = self.accessor_indices(ii as usize)?;
                        for t in idx.chunks_exact(3) {
                            let g = |k: u32| -> Result<[f32; 3], String> {
                                let v = verts
                                    .get(k as usize)
                                    .ok_or("A model index points outside its mesh.")?;
                                Ok(mat_point(&world, *v))
                            };
                            tris.push([g(t[0])?, g(t[1])?, g(t[2])?]);
                        }
                    } else {
                        for t in verts.chunks_exact(3) {
                            tris.push([
                                mat_point(&world, t[0]),
                                mat_point(&world, t[1]),
                                mat_point(&world, t[2]),
                            ]);
                        }
                    }
                    if !tris.is_empty() {
                        out.push(Mesh { tris, rgb });
                    }
                }
            }
        }
        if let Some(children) = node["children"].as_array() {
            for c in children {
                if let Some(ci) = c.as_u64() {
                    self.walk_node(ci as usize, &world, out)?;
                }
            }
        }
        Ok(())
    }
}

fn parse_glb(bytes: &[u8]) -> Result<Vec<Mesh>, String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err("That isn't a GLB (binary glTF) file.".into());
    }
    if u32le(bytes, 4) != Some(2) {
        return Err("Only glTF 2.0 files are supported.".into());
    }
    let mut json: Option<serde_json::Value> = None;
    let mut bin: &[u8] = &[];
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let len = u32le(bytes, at).unwrap_or(0) as usize;
        let kind = u32le(bytes, at + 4).unwrap_or(0);
        let data = bytes
            .get(at + 8..at + 8 + len)
            .ok_or("The GLB chunk table is damaged.")?;
        match kind {
            0x4E4F_534A => {
                json = Some(
                    serde_json::from_slice(data)
                        .map_err(|e| format!("The GLB's scene description is unreadable: {e}"))?,
                );
            }
            0x004E_4942 => bin = data,
            _ => {}
        }
        at += 8 + len;
    }
    let json = json.ok_or("The GLB has no scene description.")?;
    let g = Gltf { json, bin };
    let scene_idx = g.json["scene"].as_u64().unwrap_or(0) as usize;
    let roots = g.json["scenes"][scene_idx]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    let ident = mat_identity();
    for n in roots {
        if let Some(ni) = n.as_u64() {
            g.walk_node(ni as usize, &ident, &mut out)?;
        }
    }
    if out.is_empty() {
        return Err("No meshes found in that GLB file.".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_triangle_normalizes_to_unit_width() {
        let obj = "v 0 0 0\nv 2 0 0\nv 0 0 1\nf 1 2 3\n";
        let p = prepare_bytes(obj.as_bytes(), "obj").unwrap();
        assert_eq!(p.meshes.len(), 1);
        assert_eq!(p.meshes[0].tris.len(), 1);
        assert!((p.natural_width - 2.0).abs() < 1e-6);
        assert!((p.unit_depth - 0.5).abs() < 1e-5, "depth ratio");
        // Prepared: width spans exactly [-0.5, 0.5], base at z=0.
        let xs: Vec<f32> = p.meshes[0].tris[0].iter().map(|v| v[0]).collect();
        assert!((xs.iter().cloned().fold(f32::MIN, f32::max) - 0.5).abs() < 1e-6);
        assert!(p.meshes[0].tris.iter().flatten().all(|v| v[2] >= 0.0));
    }

    #[test]
    fn obj_quad_fans_and_negative_indices_work() {
        let obj = "v 0 0 0\nv 1 0 0\nv 1 0 1\nv 0 0 1\nf -4 -3 -2 -1\n";
        let p = prepare_bytes(obj.as_bytes(), "obj").unwrap();
        assert_eq!(p.meshes[0].tris.len(), 2, "quad fans to 2 triangles");
    }

    /// Build a minimal valid GLB around one indexed triangle mesh.
    fn sample_glb() -> Vec<u8> {
        // Positions (Y-up): a 2 m wide, 1 m tall triangle in the XY plane.
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let indices: [u16; 3] = [0, 1, 2];
        let mut bin = Vec::new();
        for v in positions {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let idx_off = bin.len();
        for i in indices {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1, "material": 0}]}],
            "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [1.0, 0.0, 0.0, 1.0]}}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": idx_off, "byteLength": 6}
            ],
            "buffers": [{"byteLength": bin.len()}]
        });
        let json_bytes = serde_json::to_vec(&json).unwrap();
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E_4942u32.to_le_bytes());
        glb.extend_from_slice(&bin);
        glb
    }

    #[test]
    fn glb_triangle_parses_with_colour_and_units() {
        let p = prepare_bytes(&sample_glb(), "glb").unwrap();
        assert_eq!(p.meshes.len(), 1);
        assert_eq!(p.meshes[0].rgb, [255, 0, 0], "base colour factor");
        assert!((p.natural_width - 2.0).abs() < 1e-5);
        // Y-up 1 m tall becomes Z-up height 1 → ratio 0.5.
        assert!((p.unit_height - 0.5).abs() < 1e-5);
    }

    #[test]
    fn junk_is_a_friendly_error() {
        assert!(prepare_bytes(b"not a model", "glb").is_err());
        assert!(prepare_bytes(b"v 0 0 0\n", "obj").is_err());
        assert!(prepare_bytes(b"", "fbx").is_err());
    }
}
