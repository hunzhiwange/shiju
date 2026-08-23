//! 主题包导入导出测试，覆盖定义与资产往返、标识冲突及 ZIP 路径安全。
//! 解包必须拒绝目录穿越条目，冲突导入则分配新标识而不覆盖现有主题。

use std::fs;
use std::path::Path;

use super::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, ThemeStore,
    export_theme_dir_to_pack, import_theme_pack_into_store,
};

fn sample_definition(id: &str) -> ThemeDefinition {
    ThemeDefinition {
        id: id.to_string(),
        name: "Ocean Dusk".to_string(),
        tagline: Some("A portable theme".to_string()),
        palette: ThemePaletteHex {
            background: "#1e1e2e".to_string(),
            text: "#cdd6f4".to_string(),
            primary: "#89b4fa".to_string(),
            success: "#a6e3a1".to_string(),
            warning: "#f9e2af".to_string(),
            danger: "#f38ba8".to_string(),
        },
        assets: ThemeAssets {
            cover: Some("cover.png".to_string()),
            background: Some("background.webp".to_string()),
            ..ThemeAssets::default()
        },
        chrome: ThemeChrome::default(),
    }
}

fn write_theme_dir(root: &Path, definition: &ThemeDefinition) {
    let theme_dir = root.join(&definition.id);
    fs::create_dir_all(&theme_dir).expect("create theme directory");
    fs::write(
        theme_dir.join("theme.json"),
        serde_json::to_vec_pretty(definition).expect("serialize theme"),
    )
    .expect("write theme definition");
    fs::write(theme_dir.join("cover.png"), b"cover bytes").expect("write cover");
    fs::write(theme_dir.join("background.webp"), b"background bytes").expect("write background");
    fs::write(theme_dir.join("not-declared.txt"), b"must not be imported")
        .expect("write unrelated file");
}

#[test]
fn theme_pack_round_trips_definition_and_assets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_root = temp.path().join("source");
    let definition = sample_definition("ocean-dusk");
    write_theme_dir(&source_root, &definition);
    let pack_path = temp.path().join("ocean-dusk.vwtheme");

    export_theme_dir_to_pack(source_root.join("ocean-dusk"), &pack_path).expect("export pack");

    let installed_root = temp.path().join("installed");
    let installed_store = ThemeStore::new(&installed_root);
    let installed =
        import_theme_pack_into_store(&pack_path, &installed_store).expect("import pack");

    assert_eq!(installed, definition);
    assert_eq!(
        fs::read(installed_root.join("ocean-dusk/cover.png")).expect("read imported cover"),
        b"cover bytes"
    );
    assert_eq!(
        fs::read(installed_root.join("ocean-dusk/background.webp"))
            .expect("read imported background"),
        b"background bytes"
    );
    assert!(!installed_root.join("ocean-dusk/not-declared.txt").exists());
}

#[test]
fn theme_pack_import_allocates_unique_id_on_conflict() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_root = temp.path().join("source");
    let definition = sample_definition("ocean-dusk");
    write_theme_dir(&source_root, &definition);
    let pack_path = temp.path().join("ocean-dusk.vwtheme");
    export_theme_dir_to_pack(source_root.join("ocean-dusk"), &pack_path).expect("export pack");

    let installed_root = temp.path().join("installed");
    let installed_store = ThemeStore::new(&installed_root);
    let existing = ThemeDefinition {
        assets: ThemeAssets::default(),
        name: "Existing Theme".to_string(),
        ..definition.clone()
    };
    installed_store.save(&existing).expect("save conflicting theme");

    let installed =
        import_theme_pack_into_store(&pack_path, &installed_store).expect("import pack");

    assert_eq!(installed.id, "ocean-dusk-2");
    assert_eq!(installed_store.load("ocean-dusk").expect("load original").name, "Existing Theme");
    assert_eq!(installed_store.load("ocean-dusk-2").expect("load imported"), installed);
}

#[test]
fn theme_pack_rejects_zip_slip_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let definition =
        ThemeDefinition { assets: ThemeAssets::default(), ..sample_definition("unsafe-theme") };
    let theme_json = serde_json::to_vec_pretty(&definition).expect("serialize theme");
    let pack_path = temp.path().join("unsafe.vwtheme");
    write_stored_zip(&pack_path, &[("theme.json", &theme_json), ("../escaped.png", b"escape")]);
    let installed_root = temp.path().join("installed");
    let installed_store = ThemeStore::new(&installed_root);

    let err = import_theme_pack_into_store(&pack_path, &installed_store)
        .expect_err("zip-slip path must be rejected");

    assert!(
        err.to_string().contains("..")
            || err.to_string().to_lowercase().contains("path")
            || err.to_string().to_lowercase().contains("relative"),
        "unexpected error: {err:#}"
    );
    assert!(!temp.path().join("escaped.png").exists());
    assert!(!installed_root.join("unsafe-theme").exists());
}

fn write_stored_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut bytes = Vec::new();
    let mut central_entries = Vec::new();

    for (name, contents) in entries {
        let offset = u32::try_from(bytes.len()).expect("zip offset fits u32");
        let crc = crc32(contents);
        push_u32(&mut bytes, 0x0403_4b50);
        push_u16(&mut bytes, 20);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u32(&mut bytes, crc);
        push_u32(&mut bytes, u32::try_from(contents.len()).expect("entry size fits u32"));
        push_u32(&mut bytes, u32::try_from(contents.len()).expect("entry size fits u32"));
        push_u16(&mut bytes, u16::try_from(name.len()).expect("name length fits u16"));
        push_u16(&mut bytes, 0);
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(contents);
        central_entries.push((*name, contents.len(), crc, offset));
    }

    let central_offset = u32::try_from(bytes.len()).expect("central offset fits u32");
    for (name, size, crc, offset) in central_entries {
        push_u32(&mut bytes, 0x0201_4b50);
        push_u16(&mut bytes, 20);
        push_u16(&mut bytes, 20);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u32(&mut bytes, crc);
        push_u32(&mut bytes, u32::try_from(size).expect("entry size fits u32"));
        push_u32(&mut bytes, u32::try_from(size).expect("entry size fits u32"));
        push_u16(&mut bytes, u16::try_from(name.len()).expect("name length fits u16"));
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u16(&mut bytes, 0);
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, offset);
        bytes.extend_from_slice(name.as_bytes());
    }

    let central_size = u32::try_from(bytes.len()).expect("zip size fits u32") - central_offset;
    push_u32(&mut bytes, 0x0605_4b50);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, u16::try_from(entries.len()).expect("entry count fits u16"));
    push_u16(&mut bytes, u16::try_from(entries.len()).expect("entry count fits u16"));
    push_u32(&mut bytes, central_size);
    push_u32(&mut bytes, central_offset);
    push_u16(&mut bytes, 0);

    fs::write(path, bytes).expect("write zip fixture");
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
