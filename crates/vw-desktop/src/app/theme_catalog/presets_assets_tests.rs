//! 随附主题预设资产测试，验证每个包定义合法且引用文件真实存在。
//! 该检查守护发布资源的完整性，避免运行时只能加载到缺图的预设。

use super::ThemeDefinition;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_PRESET_IDS: &[&str] = &[
    "blue-window-messenger",
    "cherry-frost",
    "clear-cyan",
    "cream-sage",
    "focus-capybara",
    "hacker-zero",
    "honey-milk",
    "ink-gold",
    "linen-rose",
    "mecha-cat-studio",
    "mirror-lake-ribbon",
    "moonlit-immortal",
    "neon-star-hunter",
    "peach-blush",
    "potion-workshop",
    "shanhai-nexus",
    "soft-moss",
    "starcap-teemo",
    "tahoe-beach-dawn",
    "tahoe-beach-day",
    "tahoe-beach-dusk",
    "tahoe-beach-night",
    "tahoe-dark",
    "tahoe-light",
    "vanilla-sky",
    "velvet-plum",
];

fn presets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/themes/presets")
}

fn assert_asset_exists(dir: &Path, field: &str, name: Option<&str>) {
    let Some(name) = name else {
        return;
    };
    let path = dir.join(name);
    assert!(
        path.is_file(),
        "preset `{}` declares {field}={name} but file missing at {}",
        dir.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
        path.display()
    );
}

#[test]
fn bundled_preset_packages_validate_and_assets_exist() {
    let root = presets_root();
    assert!(root.is_dir(), "expected presets directory at {}", root.display());

    let mut found = BTreeSet::new();
    for entry in fs::read_dir(&root).expect("read presets root") {
        let entry = entry.expect("read preset entry");
        if !entry.file_type().expect("file type").is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let theme_path = entry.path().join("theme.json");
        assert!(
            theme_path.is_file(),
            "missing theme.json for preset `{id}` at {}",
            theme_path.display()
        );
        let raw = fs::read_to_string(&theme_path).expect("read theme.json");
        let definition: ThemeDefinition = serde_json::from_str(&raw)
            .unwrap_or_else(|err| panic!("deserialize theme.json for `{id}` failed: {err}\n{raw}"));
        assert_eq!(definition.id, id, "theme.json id must match directory name");
        definition.validate().unwrap_or_else(|err| panic!("validate `{id}` failed: {err}"));

        assert_asset_exists(&entry.path(), "cover", definition.assets.cover.as_deref());
        assert_asset_exists(&entry.path(), "background", definition.assets.background.as_deref());
        assert_asset_exists(&entry.path(), "chat_header", definition.assets.chat_header.as_deref());
        assert!(definition.assets.cover.is_some(), "preset `{id}` must declare cover");
        assert!(definition.assets.background.is_some(), "preset `{id}` must declare background");

        found.insert(id);
    }

    let expected: BTreeSet<String> =
        EXPECTED_PRESET_IDS.iter().map(|id| (*id).to_string()).collect();
    assert_eq!(
        found,
        expected,
        "expected exactly {} bundled presets, found {}",
        expected.len(),
        found.len()
    );
    println!("validated {} presets: {:?}", found.len(), found);
}
