//! 随附主题预设管理测试，覆盖首次播种、本地修改检测和显式恢复。
//! 普通启动不得覆盖用户改动，只有已知预设的恢复操作可以重写本地副本。

use std::fs;

use super::{
    ThemeStore, bundled_preset_is_modified, ensure_bundled_presets, is_bundled_preset_id,
    list_bundled_preset_ids, restore_bundled_preset,
};

#[test]
fn ensure_bundled_presets_seeds_missing_and_skips_existing() {
    let ids = list_bundled_preset_ids();
    assert!(!ids.is_empty(), "bundled presets must be embedded via include_dir");
    let sample_id = ids[0].clone();
    assert!(
        is_bundled_preset_id(&sample_id),
        "sample id `{sample_id}` should be recognized as bundled"
    );

    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");
    let store = ThemeStore::new(&themes_root);

    let first = ensure_bundled_presets(&themes_root).expect("first ensure");
    assert!(
        first.seeded.contains(&sample_id),
        "first ensure should seed `{sample_id}`; seeded={:?}",
        first.seeded
    );
    assert!(
        !first.skipped.contains(&sample_id),
        "first ensure should not skip missing `{sample_id}`"
    );

    let loaded = store.load(&sample_id).expect("load seeded preset");
    assert_eq!(loaded.id, sample_id);
    let original_name = loaded.name.clone();
    assert!(!original_name.is_empty());

    let mut edited = loaded;
    edited.name = "user-edited-preset-name".to_string();
    store.save(&edited).expect("save edited name");

    let second = ensure_bundled_presets(&themes_root).expect("second ensure");
    assert!(
        second.skipped.contains(&sample_id),
        "second ensure should skip existing `{sample_id}`; skipped={:?}",
        second.skipped
    );
    assert!(
        !second.seeded.contains(&sample_id),
        "second ensure must not re-seed existing `{sample_id}`"
    );

    let after = store.load(&sample_id).expect("reload after second ensure");
    assert_eq!(
        after.name, "user-edited-preset-name",
        "ensure must not overwrite user-edited theme.json (bundled name was `{original_name}`)"
    );

    let theme_json = themes_root.join(&sample_id).join("theme.json");
    assert!(theme_json.is_file(), "theme.json should remain at {}", theme_json.display());
    let raw = fs::read_to_string(&theme_json).expect("read theme.json");
    assert!(raw.contains("user-edited-preset-name"), "disk theme.json must keep edited name");
}

#[test]
fn bundled_preset_is_modified_detects_local_edits() {
    let sample_id = list_bundled_preset_ids().into_iter().next().expect("bundled preset");
    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");
    ensure_bundled_presets(&themes_root).expect("seed bundled presets");

    assert!(
        !bundled_preset_is_modified(&themes_root, &sample_id).expect("check seeded preset"),
        "freshly seeded preset should match embedded bytes"
    );

    let theme_json = themes_root.join(&sample_id).join("theme.json");
    fs::write(&theme_json, b"user-edited-theme-json").expect("edit theme.json");

    assert!(
        bundled_preset_is_modified(&themes_root, &sample_id).expect("check edited preset"),
        "edited theme.json should mark the preset as modified"
    );
}

#[test]
fn restore_bundled_preset_overwrites_local_edits() {
    let sample_id = list_bundled_preset_ids().into_iter().next().expect("bundled preset");
    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");
    ensure_bundled_presets(&themes_root).expect("seed bundled presets");

    let preset_dir = themes_root.join(&sample_id);
    let theme_json = preset_dir.join("theme.json");
    let embedded_theme_json = fs::read(&theme_json).expect("read seeded theme.json");
    fs::write(&theme_json, b"user-edited-theme-json").expect("edit theme.json");
    let extra_file = preset_dir.join("user-extra.txt");
    fs::write(&extra_file, b"not bundled").expect("write extra file");

    restore_bundled_preset(&themes_root, &sample_id).expect("restore bundled preset");

    assert_eq!(
        fs::read(&theme_json).expect("read restored theme.json"),
        embedded_theme_json,
        "restore should write the embedded theme.json bytes"
    );
    assert!(!extra_file.exists(), "restore should replace the entire local preset directory");
    assert!(
        !bundled_preset_is_modified(&themes_root, &sample_id).expect("check restored preset"),
        "restored preset should match embedded bytes"
    );
}

#[test]
fn restore_bundled_preset_rejects_unknown_id() {
    let temp = tempfile::tempdir().expect("tempdir");
    let themes_root = temp.path().join("themes");

    let error = restore_bundled_preset(&themes_root, "not-a-bundled-preset")
        .expect_err("unknown preset id should be rejected");

    assert!(
        error.to_string().contains("not-a-bundled-preset"),
        "error should name the rejected preset id: {error:#}"
    );
    assert!(
        !themes_root.join("not-a-bundled-preset").exists(),
        "rejecting an unknown id must not create a local directory"
    );
}
