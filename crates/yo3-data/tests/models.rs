//! The game's 3D models (`mdl.rs`): every NPC body and avatar part reads and writes back byte-identical, and the
//! mannequin (`doll.rs`) builds on a donor body and reads back. Needs the game's files.
use yo3_data::{Ypk, default_game_dir, doll, mdl::{FrameItem, MeshPart, Model, Top}};

fn game() -> Option<Ypk> {
    let dir = std::env::var_os("YO3_GAME_DIR").map(std::path::PathBuf::from).unwrap_or_else(default_game_dir);
    Ypk::open(dir.join("package/data002.ypk")).ok()
}

#[test]
fn every_npc_body_and_avatar_part_reads_and_writes_back_the_same() {
    let Some(ypk) = game() else { return };
    let names: Vec<String> = ypk.entries().iter().map(|e| e.name.clone()).filter(|n| n.ends_with(".mdl")).collect();
    assert!(names.iter().filter(|n| n.starts_with("data/model/np")).count() >= 25, "{} models", names.len());
    for name in &names {
        let data = ypk.read(name).unwrap().unwrap();
        let model = Model::parse(&data).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(model.encode() == data, "{name}: written back differently");
    }
}

#[test]
fn the_mannequin_keeps_the_donors_skeleton_and_animations_and_wears_its_own_body() {
    let Some(ypk) = game() else { return };
    let donor = ypk.read("data/model/np005.mdl").unwrap().unwrap();
    let built = doll::mannequin(&donor).unwrap();
    let (before, mut after) = (Model::parse(&donor).unwrap(), Model::parse(&built).unwrap());
    assert_eq!(after.encode(), built);
    let animations = |m: &Model| m.items.iter().filter(|i| matches!(i, Top::Animation(_))).count();
    assert_eq!((animations(&before), animations(&after)), (53, 53), "the cut-ins' 53 animations");
    let mut meshes = vec![];
    after.each_frame(&mut |f| for item in &f.items { if let FrameItem::Mesh(m) = item { meshes.push(m.clone()) } });
    assert!(!meshes.iter().any(|m| m.name == b"faceShape" || m.name == b"hairShape"), "no face or hair");
    let body = meshes.iter().find(|m| m.name == b"bodyShape").expect("its body");
    let weighted: usize = body.parts.iter().filter_map(|p| if let MeshPart::Weights(w) = p { Some(w.values.len()) } else { None }).sum();
    assert_eq!(weighted, body.positions.len() / 12, "every corner on one joint");
    let texture = after.items.iter().find_map(|i| if let Top::Material(n, m) = i && n == b"body_Material" { Some(m.textures.clone()) } else { None });
    assert_eq!(texture, Some(vec![b".\\\\mannequin.png".to_vec()]));
}
