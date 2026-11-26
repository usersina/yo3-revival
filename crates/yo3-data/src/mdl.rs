//! The game's 3D models (`.mdl`: NPC bodies `data/model/np000.mdl`-`np024.mdl`, avatar parts), read and written:
//! [`Model::parse`] gives a tree, [`Model::encode`] writes it back byte-identical. The loader is CYO3FileLoader (yo3.exe
//! `createMesh` 0x594C00), Irrlicht's `.X` loader in binary form; the format is in docs/client-internals.md ("3D models")
//! and `tools/re/yo3mdl.py` is the same reader in Python.

pub const MAGIC: &[u8; 8] = b"\x89YO3\r\n\x1a\n";

/// A string of the format: a u8 length and its bytes (no NUL).
pub type Name = Vec<u8>;

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub items: Vec<Top>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Top {
    /// `01`: the animations' frames per second.
    Fps(u16),
    /// `02`: a named material meshes refer to.
    Material(Name, Material),
    /// `0A`: a frame (a joint), with its children and meshes.
    Frame(Frame),
    /// `28`: an animation set (the duel cut-ins play them by index).
    Animation(Animation),
    /// `00` between items.
    Stray,
    /// `FF`: the file's end.
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub face: [u8; 4],
    pub power: [u8; 4],
    pub specular: [u8; 3],
    pub emissive: [u8; 3],
    /// Each texture's name; the loader keeps the part after the last slash and loads it beside the model.
    pub textures: Vec<Name>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub name: Name,
    pub items: Vec<FrameItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameItem {
    Matrix([u8; 64]),
    Identity,
    Child(Frame),
    Mesh(Mesh),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub name: Name,
    /// Each face corner's position (unshared: every face owns its corners), 12 bytes each.
    pub positions: Vec<u8>,
    /// The faces' sizes as runs: low 3 bits the corner count (3-7), the upper bits the run length - 1.
    pub face_runs: Vec<u8>,
    pub face_sizes: Vec<u8>,
    pub parts: Vec<MeshPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MeshPart {
    /// `15`: normals and each face's normal indices (read by the loader, used by no shipped file).
    Normals { normals: Vec<u8>, faces: Vec<Vec<u16>> },
    /// `16`: a (u, v) per corner, 8 bytes each.
    Uvs(Vec<u8>),
    /// `17`: vertex colours.
    Colours(Vec<(u16, [u8; 4])>),
    /// `1A` (one material for all faces) or `18` (a byte per face): the materials, referred to or inline.
    Materials { per_face: Option<Vec<u8>>, count: u16, faces: u16, materials: Vec<MaterialRef> },
    /// `3C`: three counts, each the number of `46` entries.
    Counts([u16; 3]),
    /// `46`: one joint's skin weights.
    Weights(Weights),
}

#[derive(Clone, Debug, PartialEq)]
pub enum MaterialRef {
    Named(Name),
    Inline(Name, Material),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Weights {
    pub joint: Name,
    /// Runs of corner ids: (first, length).
    pub ids: Vec<(u16, u8)>,
    /// Each corner's weight (4 bytes, a float) and how they were run-coded: (every value stored, run length).
    pub values: Vec<[u8; 4]>,
    pub runs: Vec<(bool, u8)>,
    /// The joint's inverse bind matrix.
    pub matrix: [u8; 64],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub name: Name,
    pub items: Vec<AnimationItem>,
}

/// A key track: its type and each key's bytes as stored.
pub type Track = (u8, Vec<Vec<u8>>);

#[derive(Clone, Debug, PartialEq)]
pub enum AnimationItem {
    /// `32`: the cut-in camera's tracks (0 position, 1 target, 2 field of view).
    Camera(Vec<Track>),
    /// `29`: one joint's tracks (0 rotation, 1 scale, 2 position, 9 visibility).
    Joint(Name, Vec<Track>),
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

type Result<T> = std::result::Result<T, String>;

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or_else(|| format!("the model ends early at {:#x}", self.at))?;
        let bytes = &self.data[self.at..end];
        self.at = end;
        Ok(bytes)
    }
    fn u8(&mut self) -> Result<u8> { Ok(self.take(1)?[0]) }
    fn u16(&mut self) -> Result<u16> { Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap())) }
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> { Ok(self.take(N)?.try_into().unwrap()) }
    fn name(&mut self) -> Result<Name> { let n = self.u8()? as usize; Ok(self.take(n)?.to_vec()) }
    fn bad(&self, what: &str, tag: u8) -> String { format!("{what} tag {tag:#x} at {:#x}", self.at - 1) }
}

fn read_material(r: &mut Reader) -> Result<Material> {
    let (face, power, specular, emissive) = (r.array()?, r.array()?, r.array()?, r.array()?);
    let mut textures = vec![];
    loop {
        match r.u8()? {
            0 => break,
            3 => textures.push(r.name()?),
            t => return Err(r.bad("material", t)),
        }
    }
    Ok(Material { face, power, specular, emissive, textures })
}

fn read_mesh(r: &mut Reader) -> Result<Mesh> {
    let name = r.name()?;
    let corners = r.u16()? as usize;
    let positions = r.take(corners * 12)?.to_vec();
    let faces = r.u16()? as usize;
    let (mut face_runs, mut face_sizes) = (vec![], vec![]);
    while face_sizes.len() < faces {
        let c = r.u8()?;
        let size = c & 7;
        if !(3..=7).contains(&size) { return Err(format!("a face of {size} corners")) }
        face_runs.push(c);
        face_sizes.extend(std::iter::repeat_n(size, (c >> 3) as usize + 1));
    }
    if face_sizes.len() != faces || face_sizes.iter().map(|&s| s as usize).sum::<usize>() != corners {
        return Err(format!("{}: {faces} faces don't hold its {corners} corners", String::from_utf8_lossy(&name)));
    }
    let mut parts = vec![];
    loop {
        let t = r.u8()?;
        parts.push(match t {
            0 => break,
            0x15 => {
                let n = r.u16()? as usize;
                let normals = r.take(n * 12)?.to_vec();
                let mut faces = vec![];
                for _ in 0..r.u16()? {
                    let k = r.u16()? as usize;
                    faces.push(r.take(k * 2)?.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect());
                }
                MeshPart::Normals { normals, faces }
            }
            0x16 => { let n = r.u16()? as usize; MeshPart::Uvs(r.take(n * 8)?.to_vec()) }
            0x17 => {
                let n = r.u16()?;
                MeshPart::Colours((0..n).map(|_| Ok((r.u16()?, r.array()?))).collect::<Result<_>>()?)
            }
            0x18 | 0x1A => {
                let (count, faces) = (r.u16()?, r.u16()?);
                let per_face = if t == 0x18 { Some(r.take(faces as usize)?.to_vec()) } else { None };
                let mut materials = vec![];
                loop {
                    match r.u8()? {
                        0 => break,
                        0x19 => materials.push(MaterialRef::Named(r.name()?)),
                        2 => { let name = r.name()?; materials.push(MaterialRef::Inline(name, read_material(r)?)) }
                        s => return Err(r.bad("material list", s)),
                    }
                }
                MeshPart::Materials { per_face, count, faces, materials }
            }
            0x3C => MeshPart::Counts([r.u16()?, r.u16()?, r.u16()?]),
            0x46 => {
                let joint = r.name()?;
                let n = r.u16()? as usize;
                let (mut ids, mut counted) = (vec![], 0usize);
                while counted < n {
                    let (start, run) = (r.u16()?, r.u8()?);
                    ids.push((start, run));
                    counted += run as usize;
                }
                if counted != n { return Err(format!("{}: {counted} weight ids, not {n}", String::from_utf8_lossy(&joint))) }
                let (mut values, mut runs) = (vec![], vec![]);
                while values.len() < n {
                    let c = r.u8()?;
                    let (each, run) = (c & 0x80 != 0, c & 0x7F);
                    let first: [u8; 4] = r.array()?;
                    values.push(first);
                    for _ in 1..run { values.push(if each { r.array()? } else { first }) }
                    runs.push((each, run));
                }
                if values.len() != n { return Err("skin weights overrun".into()) }
                MeshPart::Weights(Weights { joint, ids, values, runs, matrix: r.array()? })
            }
            t => return Err(r.bad("mesh", t)),
        });
    }
    Ok(Mesh { name, positions, face_runs, face_sizes, parts })
}

fn read_frame(r: &mut Reader) -> Result<Frame> {
    let name = r.name()?;
    let mut items = vec![];
    loop {
        items.push(match r.u8()? {
            0 => break,
            0x0A => FrameItem::Child(read_frame(r)?),
            0x0B => FrameItem::Matrix(r.array()?),
            0x0C => FrameItem::Identity,
            0x14 => FrameItem::Mesh(read_mesh(r)?),
            t => return Err(r.bad("frame", t)),
        });
    }
    Ok(Frame { name, items })
}

fn read_tracks(r: &mut Reader, track_tag: u8, size: impl Fn(u8) -> Option<usize>) -> Result<Vec<Track>> {
    let mut tracks = vec![];
    loop {
        match r.u8()? {
            0 => break,
            t if t == track_tag => {
                let kind = r.u8()?;
                let n = r.u16()?;
                let size = size(kind).ok_or_else(|| format!("a key of type {kind}"))?;
                tracks.push((kind, (0..n).map(|_| Ok(r.take(size)?.to_vec())).collect::<Result<_>>()?));
            }
            t => return Err(r.bad("track", t)),
        }
    }
    Ok(tracks)
}

impl Model {
    pub fn parse(data: &[u8]) -> Result<Model> {
        if !data.starts_with(MAGIC) { return Err("not a .mdl model".into()) }
        let mut r = Reader { data, at: MAGIC.len() };
        let mut items = vec![];
        while r.at < data.len() {
            items.push(match r.u8()? {
                0xFF => {
                    if r.at != data.len() { return Err("bytes after the model's end".into()) }
                    Top::End
                }
                0 => Top::Stray,
                1 => Top::Fps(r.u16()?),
                2 => { let name = r.name()?; Top::Material(name, read_material(&mut r)?) }
                0x0A => Top::Frame(read_frame(&mut r)?),
                0x28 => {
                    let name = r.name()?;
                    let mut items = vec![];
                    loop {
                        items.push(match r.u8()? {
                            0 => break,
                            0x29 => { let joint = r.name()?; AnimationItem::Joint(joint, read_tracks(&mut r, 0x2A, |k| [18, 14, 14, 0, 0, 0, 0, 0, 0, 3].get(k as usize).copied().filter(|&s| s > 0))?) }
                            0x32 => AnimationItem::Camera(read_tracks(&mut r, 0x33, |k| [14, 14, 6].get(k as usize).copied())?),
                            t => return Err(r.bad("animation", t)),
                        });
                    }
                    Top::Animation(Animation { name, items })
                }
                t => return Err(r.bad("model", t)),
            });
        }
        Ok(Model { items })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = MAGIC.to_vec();
        for item in &self.items {
            match item {
                Top::End => { w.push(0xFF); break }
                Top::Stray => w.push(0),
                Top::Fps(fps) => { w.push(1); w.extend(fps.to_le_bytes()) }
                Top::Material(name, m) => { w.push(2); put_name(&mut w, name); put_material(&mut w, m) }
                Top::Frame(f) => { w.push(0x0A); put_frame(&mut w, f) }
                Top::Animation(a) => {
                    w.push(0x28);
                    put_name(&mut w, &a.name);
                    for item in &a.items {
                        let (track_tag, tracks) = match item {
                            AnimationItem::Joint(joint, tracks) => { w.push(0x29); put_name(&mut w, joint); (0x2A, tracks) }
                            AnimationItem::Camera(tracks) => { w.push(0x32); (0x33, tracks) }
                        };
                        for (kind, keys) in tracks {
                            w.push(track_tag);
                            w.push(*kind);
                            w.extend((keys.len() as u16).to_le_bytes());
                            keys.iter().for_each(|k| w.extend(k));
                        }
                        w.push(0);
                    }
                    w.push(0);
                }
            }
        }
        w
    }

    /// Calls `f` on every frame, depth first.
    pub fn each_frame(&mut self, f: &mut impl FnMut(&mut Frame)) {
        fn walk(frame: &mut Frame, f: &mut impl FnMut(&mut Frame)) {
            f(frame);
            for item in &mut frame.items {
                if let FrameItem::Child(c) = item { walk(c, f) }
            }
        }
        for item in &mut self.items {
            if let Top::Frame(frame) = item { walk(frame, f) }
        }
    }
}

fn put_name(w: &mut Vec<u8>, name: &[u8]) {
    w.push(name.len() as u8);
    w.extend(name);
}

fn put_material(w: &mut Vec<u8>, m: &Material) {
    w.extend(m.face);
    w.extend(m.power);
    w.extend(m.specular);
    w.extend(m.emissive);
    for t in &m.textures { w.push(3); put_name(w, t) }
    w.push(0);
}

fn put_frame(w: &mut Vec<u8>, f: &Frame) {
    put_name(w, &f.name);
    for item in &f.items {
        match item {
            FrameItem::Child(c) => { w.push(0x0A); put_frame(w, c) }
            FrameItem::Matrix(m) => { w.push(0x0B); w.extend(m) }
            FrameItem::Identity => w.push(0x0C),
            FrameItem::Mesh(m) => { w.push(0x14); put_mesh(w, m) }
        }
    }
    w.push(0);
}

fn put_mesh(w: &mut Vec<u8>, m: &Mesh) {
    put_name(w, &m.name);
    w.extend(((m.positions.len() / 12) as u16).to_le_bytes());
    w.extend(&m.positions);
    w.extend((m.face_sizes.len() as u16).to_le_bytes());
    w.extend(&m.face_runs);
    for part in &m.parts {
        match part {
            MeshPart::Normals { normals, faces } => {
                w.push(0x15);
                w.extend(((normals.len() / 12) as u16).to_le_bytes());
                w.extend(normals);
                w.extend((faces.len() as u16).to_le_bytes());
                for f in faces { w.extend((f.len() as u16).to_le_bytes()); f.iter().for_each(|i| w.extend(i.to_le_bytes())) }
            }
            MeshPart::Uvs(uvs) => { w.push(0x16); w.extend(((uvs.len() / 8) as u16).to_le_bytes()); w.extend(uvs) }
            MeshPart::Colours(c) => { w.push(0x17); w.extend((c.len() as u16).to_le_bytes()); c.iter().for_each(|(i, rgba)| { w.extend(i.to_le_bytes()); w.extend(rgba) }) }
            MeshPart::Materials { per_face, count, faces, materials } => {
                w.push(if per_face.is_some() { 0x18 } else { 0x1A });
                w.extend(count.to_le_bytes());
                w.extend(faces.to_le_bytes());
                if let Some(idx) = per_face { w.extend(idx) }
                for m in materials {
                    match m {
                        MaterialRef::Named(n) => { w.push(0x19); put_name(w, n) }
                        MaterialRef::Inline(n, mat) => { w.push(2); put_name(w, n); put_material(w, mat) }
                    }
                }
                w.push(0);
            }
            MeshPart::Counts(c) => { w.push(0x3C); c.iter().for_each(|v| w.extend(v.to_le_bytes())) }
            MeshPart::Weights(wt) => {
                w.push(0x46);
                put_name(w, &wt.joint);
                w.extend((wt.values.len() as u16).to_le_bytes());
                for (start, run) in &wt.ids { w.extend(start.to_le_bytes()); w.push(*run) }
                let mut i = 0;
                for &(each, run) in &wt.runs {
                    w.push(u8::from(each) << 7 | run);
                    w.extend(wt.values[i]);
                    if each { for k in 1..run as usize { w.extend(wt.values[i + k]) } }
                    i += run as usize;
                }
                w.extend(wt.matrix);
            }
        }
    }
    w.push(0);
}
