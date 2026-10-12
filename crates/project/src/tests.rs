use effectcraft_keyframe::{Keyframe, ShapePath, Value};
use effectcraft_time::{FrameRate, Tick};

use crate::build::{self, Ids};
use crate::{Comp, ItemKind, LayerSource, MaskMode, Project, Solid};

fn project_with_layer() -> (Project, crate::ItemId) {
    let mut p = Project::default();
    let comp = Comp::new(1920, 1080, FrameRate::FPS_29_97, Tick::from_seconds_f64(10.0));
    let cid = p.add_item("Main", effectcraft_color::Label::Sandstone, None, ItemKind::Comp(comp.clone().into()));
    let sid = p.add_item(
        "Red Solid",
        effectcraft_color::Label::Red,
        None,
        ItemKind::Solid(Solid { color: [1.0, 0.0, 0.0], width: 1920, height: 1080, pixel_aspect: 1.0 }),
    );
    let layer = build::layer(&mut p, &comp, "Red Solid 1", LayerSource::Solid { item: sid }, (1920, 1080), None);
    p.comp_mut(cid).unwrap().layers.push(layer);
    (p, cid)
}

#[test]
fn paths_resolve() {
    let (p, cid) = project_with_layer();
    let l = &p.comp(cid).unwrap().layers[0];
    assert_eq!(l.props.prop("transform/position").unwrap().value, Value::Vec3([960.0, 540.0, 0.0]));
    assert_eq!(l.props.prop("Transform/Opacity").unwrap().value, Value::Scalar(100.0));
    assert_eq!(l.props.prop("transform/#7").unwrap().match_id, "rotation");
    let uid = l.props.prop("transform/scale").unwrap().uid;
    let path = l.props.path_of(uid).unwrap();
    assert_eq!(l.props.prop(&path).unwrap().match_id, "scale");
    assert_eq!(l.props.name_path_of(uid).unwrap(), "Transform/Scale");
}

#[test]
fn masks_and_occurrence_paths() {
    let (mut p, cid) = project_with_layer();
    let mut next = p.next_id;
    let m1 = build::mask(&mut Ids(&mut next), "Mask 1", ShapePath::rect([0.0, 0.0], 10.0, 10.0), MaskMode::Add, [255, 0, 0]);
    let m2 = build::mask(&mut Ids(&mut next), "Mask 2", ShapePath::ellipse([0.0, 0.0], 10.0, 10.0), MaskMode::Subtract, [0, 255, 0]);
    p.next_id = next;
    let l = &mut p.comp_mut(cid).unwrap().layers[0];
    let masks = l.props.sub_mut("masks").unwrap();
    masks.children.push(m1.into());
    masks.children.push(m2.into());
    assert_eq!(l.props.group("masks/mask#2").unwrap().name, "Mask 2");
    assert_eq!(l.props.group("masks/Mask 2").unwrap().name, "Mask 2");
    assert!(l.props.prop("masks/mask#2/feather").is_some());
}

#[test]
fn animate_and_roundtrip_json() {
    let (mut p, cid) = project_with_layer();
    {
        let l = &mut p.comp_mut(cid).unwrap().layers[0];
        let op = l.props.prop_mut("transform/opacity").unwrap();
        op.set_animated(true, Tick::ZERO);
        op.set_value_at(Tick::from_seconds_f64(1.0), Value::Scalar(0.0));
        assert_eq!(op.keys.len(), 2);
        assert_eq!(op.value_at(Tick::from_seconds_f64(0.5)), Value::Scalar(50.0));
    }
    let json = p.to_json();
    let q = Project::from_json(&json).unwrap();
    assert_eq!(p, q);
    let _ = Keyframe::new(Tick::ZERO, Value::Scalar(0.0));
}

#[test]
fn unique_names_and_cycles() {
    let (p, cid) = project_with_layer();
    let c = p.comp(cid).unwrap();
    assert_eq!(c.unique_layer_name("Red Solid 1"), "Red Solid 2");
    assert_eq!(c.unique_layer_name("Shape Layer 1"), "Shape Layer 1");
    assert!(p.comp_contains(cid, cid));
}

fn nested_comp(p: &mut Project, source: crate::ItemId, name: &str) -> crate::ItemId {
    let comp = Comp::new(16, 16, FrameRate::FPS_29_97, Tick::from_seconds_f64(1.0));
    let mut layer = build::layer(p, &comp, name, LayerSource::Comp { item: source }, (16, 16), None);
    layer.name = name.into();
    let mut comp = comp;
    comp.layers.push(layer);
    p.add_item(name, effectcraft_color::Label::Sandstone, None, ItemKind::Comp(comp.into()))
}

#[test]
fn cyclic_project_files_are_refused_and_reachability_terminates() {
    let (mut p, a) = project_with_layer();
    let b = nested_comp(&mut p, a, "B");
    let independent = p.add_item("C", effectcraft_color::Label::Sandstone, None, ItemKind::Comp(Comp::new(16, 16, FrameRate::FPS_29_97, Tick::ZERO).into()));
    p.comp_mut(a).unwrap().layers[0].source = LayerSource::Comp { item: b };
    assert!(!p.comp_contains(a, independent));
    assert!(p.comp_contains(a, b));
    assert!(Project::from_json(&p.to_json()).unwrap_err().to_string().contains("cycle"));
}

#[test]
fn missing_composition_references_are_refused() {
    let (mut p, a) = project_with_layer();
    p.comp_mut(a).unwrap().layers[0].source = LayerSource::Comp { item: crate::ItemId(999) };
    assert!(Project::from_json(&p.to_json()).unwrap_err().to_string().contains("missing composition"));
}

#[test]
fn hostile_id_counters_and_object_ids_are_refused_without_overflow() {
    let (p, a) = project_with_layer();
    for next in [u64::MAX, crate::MAX_NEXT_ID + 1] {
        let mut bad = p.clone();
        bad.next_id = next;
        assert!(Project::from_json(&bad.to_json()).is_err());
    }
    let mut bad = p.clone();
    let mut item = bad.items.remove(&a).unwrap();
    item.id = crate::ItemId(u64::MAX);
    bad.items.insert(item.id, item);
    assert!(Project::from_json(&bad.to_json()).is_err());
    let mut bad = p.clone();
    bad.comp_mut(a).unwrap().layers[0].props.uid = u64::MAX;
    assert!(Project::from_json(&bad.to_json()).is_err());
    let mut bad = p;
    bad.items.get_mut(&a).unwrap().id = crate::ItemId(777);
    assert!(Project::from_json(&bad.to_json()).is_err());
}

#[test]
fn a_stale_counter_is_repaired_and_an_invalid_builder_counter_never_wraps() {
    let (mut p, _) = project_with_layer();
    p.next_id = 1;
    let repaired = Project::from_json(&p.to_json()).unwrap();
    assert!(repaired.next_id > p.items.keys().map(|id| id.0).max().unwrap());
    p.next_id = u64::MAX;
    p.alloc();
    assert_eq!(p.next_id, u64::MAX);
    assert!(p.check_id_counter().is_err());
}

#[test]
fn shared_and_deep_composition_graphs_are_walked_iteratively() {
    let (mut p, first) = project_with_layer();
    let mut last = first;
    for index in 0..256 {
        last = nested_comp(&mut p, last, &format!("Nested {index}"));
    }
    assert!(p.comp_contains(last, first));
    assert!(!p.comp_contains(first, last));
    p.check().unwrap();
    Project::from_json(&p.to_json()).unwrap();
}

#[test]
fn camera_and_light_layers_build() {
    let (mut p, cid) = project_with_layer();
    let comp = p.comp(cid).unwrap().clone();
    let cam = build::layer(&mut p, &comp, "Camera 1", LayerSource::Camera, (0, 0), None);
    assert!(cam.props.prop("cameraOptions/zoom").is_some());
    assert!(cam.props.prop("transform/poi").is_some());
    let light = build::layer(&mut p, &comp, "Light 1", LayerSource::Light { kind: crate::LightKind::Spot }, (0, 0), None);
    assert!(light.props.prop("lightOptions/coneAngle").is_some());
}

#[test]
fn same_content_sees_allocated_ids_and_comp_edits() {
    let (p, cid) = project_with_layer();
    let mut q = p.clone();
    assert!(q.same_content(&p));
    q.alloc();
    assert!(!q.same_content(&p), "an allocated id stays taken");
    let mut q = p.clone();
    // A comp copied on write but left equal is the same; a changed one is not.
    let _ = q.comp_mut(cid).unwrap();
    assert!(q.same_content(&p));
    q.comp_mut(cid).unwrap().layers[0].name = "Renamed".into();
    assert!(!q.same_content(&p));
    let mut r = p.clone();
    r.item_mut(cid).unwrap().name = "Other".into();
    assert!(!r.same_content(&p));
    let mut r = p.clone();
    r.settings.bit_depth = crate::BitDepth::Bpc16;
    assert!(!r.same_content(&p));
}

#[test]
fn project_files_name_their_version_and_always_read_back() {
    let (mut p, cid) = project_with_layer();
    let text = p.to_file_json().unwrap();
    assert_eq!(crate::saved_by(&text).as_deref(), Some(crate::APP_VERSION));
    assert!(text.trim_start().starts_with("{\n  \"savedBy\""), "the version comes first");
    assert_eq!(Project::from_json(&text).unwrap(), p);
    assert_eq!(crate::saved_by(&p.to_json()), None, "plain JSON (worker hand-offs) has none");
    // JSON can't hold NaN: saving refuses instead of writing a file that can't be opened.
    p.comp_mut(cid).unwrap().layers[0].props.prop_mut("transform/opacity").unwrap().value = Value::Scalar(f64::NAN);
    let e = p.to_file_json().unwrap_err().to_string();
    assert!(e.contains("can't be saved"), "{e}");
    assert!(crate::is_newer_version("0.2.0", "0.1.9") && crate::is_newer_version("1.0.0-rc.1", "0.9.9"));
    assert!(!crate::is_newer_version("0.1.1", "0.1.1") && !crate::is_newer_version("0.1.0", "0.1.1"));
}
