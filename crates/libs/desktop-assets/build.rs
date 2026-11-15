//! 生成命名图标目录。
use serde_json::json;
use std::collections::BTreeSet;
use std::{env, fs};
use std::path::{Path, PathBuf};

fn sanitize_icon_token(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(".svg");
    if trimmed.is_empty() {
        return None;
    }
    let mut normalized = String::with_capacity(trimmed.len());
    let mut previous_separator = false;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_separator = false;
        } else if matches!(ch, '-' | '_' | ' ') {
            if !previous_separator && !normalized.is_empty() {
                normalized.push('-');
                previous_separator = true;
            }
        } else {
            return None;
        }
    }
    while normalized.ends_with('-') {
        normalized.pop();
    }
    if normalized.is_empty() { None } else { Some(normalized) }
}

fn collect_named_icon_family_entries(
    dir: &Path,
    family: &str,
    output: &mut BTreeSet<String>,
    depth: usize,
) {
    if depth > 1 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_named_icon_family_entries(&path, family, output, depth + 1);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("svg") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let mut name = sanitize_icon_token(stem).unwrap_or_default();
        if family == "phosphor" {
            for suffix in ["-thin", "-light", "-bold", "-fill", "-duotone"] {
                if let Some(stripped) = name.strip_suffix(suffix) {
                    name = stripped.to_string();
                    break;
                }
            }
        }
        if !name.is_empty() {
            output.insert(name);
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));


    let icon_assets_dir = manifest_dir.join("../../../assets/icons");
    println!("cargo:rerun-if-changed={}", icon_assets_dir.display());

    let icon_index_dir = out_dir.join("named_icon_index");
    fs::create_dir_all(&icon_index_dir).expect("create named icon index dir");

    let mut families_json = Vec::new();
    let mut family_include_rows = Vec::new();

    let entries = fs::read_dir(&icon_assets_dir).expect("read icons assets dir");
    let mut family_paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    family_paths.sort();

    for path in family_paths {
        let Some(family_raw) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(family) = sanitize_icon_token(family_raw) else {
            continue;
        };

        let mut icons = BTreeSet::new();
        collect_named_icon_family_entries(&path, &family, &mut icons, 0);
        if icons.is_empty() {
            continue;
        }

        let icons = icons.into_iter().collect::<Vec<_>>();
        let family_json_path = icon_index_dir.join(format!("{family}.json"));
        let family_json = json!({
            "family": family.clone(),
            "icons": icons.clone(),
        });
        fs::write(
            &family_json_path,
            serde_json::to_vec(&family_json).expect("serialize family icon json"),
        )
        .expect("write family icon json");

        families_json.push(family_json);
        family_include_rows.push(format!(
            "    (\"{family}\", include_str!(concat!(env!(\"OUT_DIR\"), \"/named_icon_index/{family}.json\"))),"
        ));
    }

    fs::write(
        icon_index_dir.join("catalog.json"),
        serde_json::to_vec(&families_json).expect("serialize named icon catalog json"),
    )
    .expect("write named icon catalog json");

    let generated_rs = format!(
        "pub static NAMED_ICON_CATALOG_JSON: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/named_icon_index/catalog.json\"));\n\
/// NAMED_ICON_FAMILY_JSONS 是该模块对外使用的常量值。
pub static NAMED_ICON_FAMILY_JSONS: &[(&str, &str)] = &[\n{}\n];\n",
        family_include_rows.join("\n")
    );
    fs::write(out_dir.join("named_icon_generated.rs"), generated_rs)
        .expect("write named icon generated include");
}
