//! Our own NPC body: a clothes-store mannequin (an egg head, a sculpted torso, tapered limbs with ball joints, paddle
//! hands), each part our own geometry rigidly on one joint of a donor body the game has. The donor's skeleton and its 53
//! animations stay (the duel cut-ins play them by index), so it moves like any NPC; its own meshes go. The donor is
//! the player's own game file, so the body is built on their machine (the launcher) and no Konami data ships.
//!
//! Its shading is painted, as the game's NPC skins are: each corner's UV is its smooth normal seen from the front, on a
//! lit ivory sphere ([`shading_png`], a "matcap").

use crate::mdl::{Frame, FrameItem, MaterialRef, Mesh, MeshPart, Model, Top, Weights};
use std::collections::HashMap;

/// The texture the mannequin's material names, beside its model.
pub const TEXTURE: &str = "mannequin.png";
/// The donor's meshes that go with its face and hair (the mannequin has neither).
const DROP: [&[u8]; 7] = [b"faceShape", b"hairShape", b"eyesShape", b"millenniumeyeShape", b"ha_upperShape", b"ha_lowerShape", b"kuchiShape"];
const BODY: &[u8] = b"bodyShape";
const BODY_MATERIAL: &[u8] = b"body_Material";

type V3 = [f64; 3];
fn add(a: V3, b: V3) -> V3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn mul(a: V3, k: f64) -> V3 { [a[0] * k, a[1] * k, a[2] * k] }
fn dot(a: V3, b: V3) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn unit(a: V3) -> V3 { let n = dot(a, a).sqrt(); if n > 0.0 { mul(a, 1.0 / n) } else { a } }

type Faces = Vec<Vec<V3>>;

/// A closed grid `point(u, v)` (u around, v from one pole to the other): triangles at the poles, quads between.
fn surface(point: impl Fn(f64, f64) -> V3, nu: usize, nv: usize) -> Faces {
    let grid: Vec<Vec<V3>> = (0..=nv).map(|j| (0..nu).map(|i| point(i as f64 / nu as f64, j as f64 / nv as f64)).collect()).collect();
    let mut faces = vec![];
    for j in 0..nv {
        for i in 0..nu {
            let (a, b, c, d) = (grid[j][i], grid[j][(i + 1) % nu], grid[j + 1][(i + 1) % nu], grid[j + 1][i]);
            faces.push(if j == 0 { vec![a, c, d] } else if j == nv - 1 { vec![a, b, c] } else { vec![a, b, c, d] });
        }
    }
    faces
}

/// An ellipsoid centred on `c`, `egg` narrowing its lower half (a chin).
fn ellipsoid(c: V3, r: V3, egg: f64, nu: usize, nv: usize) -> Faces {
    surface(|u, v| {
        let (th, ph) = (std::f64::consts::TAU * u, std::f64::consts::PI * v);
        let (y, s) = (ph.cos(), ph.sin());
        let k = 1.0 - egg * (-y).max(0.0);
        add(c, [r[0] * s * th.cos() * k, r[1] * y, r[2] * s * th.sin() * k])
    }, nu, nv)
}

/// A capsule from `a` (radius `r1`) to `b` (radius `r2`), rounded at both ends.
fn capsule(a: V3, b: V3, r1: f64, r2: f64) -> Faces {
    let axis = sub(b, a);
    let len = dot(axis, axis).sqrt();
    let w = unit(axis);
    let t = unit(cross(w, if w[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] }));
    let q = cross(w, t);
    surface(|u, v| {
        let th = std::f64::consts::TAU * u;
        let (along, rad) = if v < 1.0 / 3.0 {
            let ang = v * 3.0 * std::f64::consts::FRAC_PI_2;
            (-r1 * ang.cos(), r1 * ang.sin())
        } else if v > 2.0 / 3.0 {
            let ang = (1.0 - v) * 3.0 * std::f64::consts::FRAC_PI_2;
            (len + r2 * ang.cos(), r2 * ang.sin())
        } else {
            let f = (v - 1.0 / 3.0) * 3.0;
            (f * len, r1 + (r2 - r1) * f)
        };
        add(add(a, mul(w, along)), mul(add(mul(t, th.cos()), mul(q, th.sin())), rad))
    }, 12, 12)
}

/// The mannequin's parts in the donor's bind pose (its units: about 172 tall, feet at y 0, the face towards -z, +x the
/// character's left), by joint: the left side's are mirrored onto the right.
fn parts() -> Vec<(String, Faces)> {
    let ball = |c: V3, r: f64| ellipsoid(c, [r, r, r], 0.0, 12, 8);
    let left: Vec<(&str, Faces)> = vec![
        ("neck_face", ellipsoid([0.0, 160.5, -0.8], [7.6, 10.2, 8.8], 0.22, 16, 10)),
        ("neck", capsule([0.0, 145.5, 0.4], [0.0, 153.0, -0.2], 3.4, 3.1)),
        ("upper_02", ellipsoid([0.0, 130.5, -3.2], [14.0, 14.6, 8.2], 0.0, 16, 10)),     // chest
        ("upper_02", ellipsoid([0.0, 140.0, -2.0], [16.0, 3.6, 6.8], 0.0, 16, 10)),      // the shoulders' line
        ("upper_01", ellipsoid([0.0, 117.5, -3.6], [9.6, 10.5, 6.8], 0.0, 16, 10)),      // waist
        ("under", ellipsoid([0.0, 106.0, -3.6], [13.4, 10.6, 8.6], 0.0, 16, 10)),        // hips
        ("L_arm_01", ellipsoid([12.6, 141.3, 1.0], [4.6, 4.6, 4.6], 0.0, 12, 8)),        // shoulder ball
        ("L_arm_01", capsule([14.5, 141.3, 2.1], [35.0, 141.3, 2.1], 4.5, 3.7)),
        ("L_arm_02", ball([37.5, 141.3, 2.1], 3.9)),                                     // elbow ball
        ("L_arm_02", capsule([40.0, 141.3, 2.1], [60.5, 141.3, 2.1], 3.6, 2.7)),
        ("L_hand", ellipsoid([68.0, 141.3, 2.1], [5.4, 1.9, 3.3], 0.0, 12, 8)),
        ("L_leg_01", ball([7.4, 100.5, -3.7], 5.4)),                                     // hip ball
        ("L_leg_01", capsule([7.4, 98.0, -3.6], [7.4, 59.5, -2.3], 5.7, 4.0)),
        ("L_leg_03", ball([7.4, 56.5, -2.2], 3.9)),                                      // knee ball
        ("L_leg_03", capsule([7.4, 53.5, -2.0], [7.4, 14.5, 1.8], 3.8, 2.5)),
        ("L_foot_01", ellipsoid([7.4, 5.2, -2.6], [3.6, 3.6, 8.8], 0.3, 16, 10)),
    ];
    let mut out = vec![];
    for (joint, faces) in left {
        if let Some(side) = joint.strip_prefix("L_") {
            let mirrored = faces.iter().map(|f| f.iter().rev().map(|p| [-p[0], p[1], p[2]]).collect()).collect();
            out.push((joint.to_string(), faces));
            out.push((format!("R_{side}"), mirrored));
        } else {
            out.push((joint.to_string(), faces));
        }
    }
    out
}

/// The mannequin as the mesh replacing the donor's body: corners, UVs from smooth normals, one material, and each
/// part's corners weighted 1 to its joint with the donor's inverse bind matrix.
fn body_mesh(bind: &HashMap<Vec<u8>, [u8; 64]>) -> Result<Mesh, String> {
    let (mut positions, mut uvs, mut sizes) = (vec![], vec![], vec![]);
    let mut groups: Vec<(Vec<u8>, Vec<u16>)> = vec![];
    for (joint, faces) in parts() {
        let corners: Vec<V3> = faces.iter().flatten().copied().collect();
        let centre = mul(corners.iter().fold([0.0; 3], |a, &b| add(a, b)), 1.0 / corners.len() as f64);
        let key = |p: V3| [(p[0] * 1000.0).round() as i64, (p[1] * 1000.0).round() as i64, (p[2] * 1000.0).round() as i64];
        let outward = |f: &[V3]| {
            let n = unit(cross(sub(f[1], f[0]), sub(f[2], f[0])));
            let mid = mul(f.iter().fold([0.0; 3], |a, &b| add(a, b)), 1.0 / f.len() as f64);
            if dot(n, sub(mid, centre)) < 0.0 { mul(n, -1.0) } else { n }
        };
        let mut normal: HashMap<[i64; 3], V3> = HashMap::new();
        for f in &faces {
            let n = outward(f);
            for &p in f { let e = normal.entry(key(p)).or_insert([0.0; 3]); *e = add(*e, n) }
        }
        let ids = match groups.iter_mut().find(|(j, _)| j.as_slice() == joint.as_bytes()) {
            Some((_, ids)) => ids,
            None => { groups.push((joint.as_bytes().to_vec(), vec![])); &mut groups.last_mut().unwrap().1 }
        };
        for f in &faces {
            // The file holds each face clockwise seen from outside (the loader reverses it).
            let n = cross(sub(f[1], f[0]), sub(f[2], f[0]));
            let mid = mul(f.iter().fold([0.0; 3], |a, &b| add(a, b)), 1.0 / f.len() as f64);
            let order: Vec<V3> = if dot(n, sub(mid, centre)) > 0.0 { f.iter().rev().copied().collect() } else { f.clone() };
            for p in order {
                let id = u16::try_from(positions.len() / 12).map_err(|_| "the mannequin has too many corners")?;
                ids.push(id);
                p.iter().for_each(|&x| positions.extend((x as f32).to_le_bytes()));
                let n = unit(normal[&key(p)]);
                // Seen from the front (the face looks to -z): the lit sphere's x to the right, y up.
                uvs.extend((0.5 - 0.48 * n[0] as f32).to_le_bytes());
                uvs.extend((0.5 - 0.48 * n[1] as f32).to_le_bytes());
            }
            sizes.push(f.len() as u8);
        }
    }
    let mut face_runs = vec![];
    let mut i = 0;
    while i < sizes.len() {
        let run = sizes[i..].iter().take_while(|&&s| s == sizes[i]).count().min(32);
        face_runs.push(((run - 1) as u8) << 3 | sizes[i]);
        i += run;
    }
    let mut parts = vec![
        MeshPart::Uvs(uvs),
        MeshPart::Materials { per_face: None, count: 1, faces: sizes.len() as u16, materials: vec![MaterialRef::Named(BODY_MATERIAL.to_vec())] },
        MeshPart::Counts([groups.len() as u16; 3]),
    ];
    for (joint, ids) in groups {
        let matrix = *bind.get(&joint).ok_or_else(|| format!("the donor has no joint {}", String::from_utf8_lossy(&joint)))?;
        let mut runs = vec![];
        let mut s = 0;
        while s < ids.len() {
            let run = ids[s..].iter().enumerate().take_while(|&(k, &id)| id as usize == ids[s] as usize + k).count().min(255);
            runs.push((ids[s], run as u8));
            s += run;
        }
        let mut value_runs = vec![];
        let mut left = ids.len();
        while left > 0 { let r = left.min(127); value_runs.push((false, r as u8)); left -= r }
        parts.push(MeshPart::Weights(Weights { joint, ids: runs, values: vec![1.0f32.to_le_bytes(); ids.len()], runs: value_runs, matrix }));
    }
    Ok(Mesh { name: BODY.to_vec(), positions, face_runs, face_sizes: sizes, parts })
}

/// The mannequin built on `donor` (a game NPC body's `.mdl`): its skeleton, animations, duel disk and cards kept, its
/// body replaced, its face and hair gone, its body material on [`TEXTURE`] with a soft gloss.
pub fn mannequin(donor: &[u8]) -> Result<Vec<u8>, String> {
    let mut model = Model::parse(donor)?;
    let mut bind: HashMap<Vec<u8>, [u8; 64]> = HashMap::new();
    model.each_frame(&mut |frame: &mut Frame| {
        for item in &frame.items {
            if let FrameItem::Mesh(mesh) = item {
                for part in &mesh.parts {
                    if let MeshPart::Weights(w) = part { bind.entry(w.joint.clone()).or_insert(w.matrix); }
                }
            }
        }
    });
    let body = body_mesh(&bind)?;
    let mut replaced = false;
    model.each_frame(&mut |frame: &mut Frame| {
        frame.items.retain(|item| !matches!(item, FrameItem::Mesh(m) if DROP.contains(&m.name.as_slice())));
        for item in &mut frame.items {
            if let FrameItem::Mesh(m) = item && m.name == BODY { *m = body.clone(); replaced = true }
        }
    });
    if !replaced { return Err("the donor has no bodyShape".into()) }
    for item in &mut model.items {
        if let Top::Material(name, material) = item && name == BODY_MATERIAL {
            material.textures = vec![format!(".\\\\{TEXTURE}").into_bytes()];
            material.power = 40.0f32.to_le_bytes();
            material.specular = [90; 3];
        }
    }
    Ok(model.encode())
}

/// The colour of the mannequin's surface facing `n` (a unit normal seen from the front): its matcap's shading.
fn ivory(n: V3) -> [f64; 3] {
    let light = unit([-0.45, 0.55, 0.70]);
    let half = unit(add(light, [0.0, 0.0, 1.0]));
    let diffuse = dot(n, light).clamp(0.0, 1.0);
    let spec = dot(n, half).clamp(0.0, 1.0).powi(40);
    let shade = 0.42 + 0.50 * diffuse + 0.10 * n[2].max(0.0);
    [240.0, 231.0, 214.0].map(|base| (base * shade + 255.0 * 0.35 * spec).clamp(0.0, 255.0))
}

/// A bust of the mannequin in a `size` square (an egg head over a neck and shoulders, shaded as its body), on `background`
/// (RGBA per pixel from its position, 0-1 each way): the picture its faces and portrait show until it has its own.
fn bust(size: usize, scale: f64, top: f64, background: impl Fn(f64, f64) -> [u8; 4]) -> Vec<u8> {
    // Ellipsoids in the picture's units (x right, y down, 0-1): head, neck, shoulders.
    let shapes = [([0.5, top + 0.22 * scale, 0.0], [0.15 * scale, 0.21 * scale, 0.17 * scale]),
        ([0.5, top + 0.46 * scale, 0.0], [0.075 * scale, 0.12 * scale, 0.075 * scale]),
        ([0.5, top + 0.86 * scale, 0.0], [0.38 * scale, 0.30 * scale, 0.20 * scale]),
        ([0.5, top + 1.10 * scale, 0.0], [0.30 * scale, 0.30 * scale, 0.18 * scale])];
    let mut rgba = Vec::with_capacity(size * size * 4);
    for py in 0..size {
        for px in 0..size {
            let (x, y) = ((px as f64 + 0.5) / size as f64, (py as f64 + 0.5) / size as f64);
            // The nearest surface towards the viewer at this pixel, its normal (y up for the light).
            let mut best: Option<(f64, V3)> = None;
            for (c, r) in shapes {
                let (dx, dy) = ((x - c[0]) / r[0], (y - c[1]) / r[1]);
                let d = 1.0 - dx * dx - dy * dy;
                if d <= 0.0 { continue }
                let dz = d.sqrt();
                let front = dz * r[2];
                if best.is_none_or(|(f, _)| front > f) { best = Some((front, unit([dx / r[0], -dy / r[1], dz / r[2]]))) }
            }
            match best {
                Some((_, n)) => { let c = ivory(n); rgba.extend([c[0] as u8, c[1] as u8, c[2] as u8, 255]) }
                None => rgba.extend(background(x, y)),
            }
        }
    }
    png_rgba(size, &rgba)
}

/// A duel face (128x128, `data/face/np025f<k>.png`): the bust on a dusk gradient. Faceless, so the five expressions are
/// the same picture.
pub fn face_png() -> Vec<u8> {
    bust(128, 1.3, -0.08, |_, y| [(70.0 + 60.0 * y) as u8, (60.0 + 30.0 * y) as u8, (110.0 - 30.0 * y) as u8, 255])
}

/// Kaiba Corporation's portrait (512x512, `data/npc/npcsel025.png`): the bust on transparency, as the game's are.
pub fn portrait_png() -> Vec<u8> {
    bust(512, 0.95, 0.12, |_, _| [0, 0, 0, 0])
}

fn png_rgba(size: usize, rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    {
        let mut encoder = png::Encoder::new(&mut out, size as u32, size as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a PNG header in memory");
        writer.write_image_data(rgba).expect("a PNG in memory");
    }
    out
}

/// The mannequin's shading: an ivory sphere lit from the upper left front, a soft highlight and a darker rim, 256x256.
pub fn shading_png() -> Vec<u8> {
    const SIZE: usize = 256;
    let mut rgba = Vec::with_capacity(SIZE * SIZE * 4);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let nx = (x as f64 + 0.5) / SIZE as f64 * 2.0 - 1.0;
            let ny = 1.0 - (y as f64 + 0.5) / SIZE as f64 * 2.0;
            let nz = (1.0 - nx * nx - ny * ny).clamp(0.0, 1.0).sqrt();
            let c = ivory([nx, ny, nz]);
            rgba.extend([c[0] as u8, c[1] as u8, c[2] as u8, 255]);
        }
    }
    png_rgba(SIZE, &rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_part_is_closed_and_mirrored() {
        let p = parts();
        assert_eq!(p.iter().filter(|(j, _)| j.starts_with("L_")).count(), p.iter().filter(|(j, _)| j.starts_with("R_")).count());
        assert!(p.iter().all(|(_, faces)| faces.iter().all(|f| (3..=4).contains(&f.len()))));
        for png in [shading_png(), face_png(), portrait_png()] { assert!(png.starts_with(b"\x89PNG")) }
        // YO3_DOLL_PREVIEW=DIR writes the pictures there to look at.
        if let Some(dir) = std::env::var_os("YO3_DOLL_PREVIEW") {
            let dir = std::path::PathBuf::from(dir);
            for (name, png) in [("shading.png", shading_png()), ("face.png", face_png()), ("portrait.png", portrait_png())] {
                std::fs::write(dir.join(name), png).unwrap();
            }
        }
    }
}
