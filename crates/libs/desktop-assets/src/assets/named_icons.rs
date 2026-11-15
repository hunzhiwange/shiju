//! 提供桌面端命名图标查找。
//! 模块将用户可读图标名映射到内置资源，避免 UI 层直接依赖文件名。

use iced::{Color, widget::image};
use include_dir::{Dir, include_dir};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::HashMap;

static NAMED_ICON_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../../assets/icons");

static NAMED_ICON_IMAGES: Lazy<Mutex<HashMap<String, image::Handle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static NAMED_ICON_FAMILY_JSONS: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| super::named_icon_generated::NAMED_ICON_FAMILY_JSONS.iter().copied().collect());

/// NamedIconFamily 表示该模块对外暴露的结构化状态。
#[derive(Debug, Clone, Deserialize)]
pub struct NamedIconFamily {
    /// family 字段保存该结构体对外暴露的同名状态。
    pub family: String,
    /// icons 字段保存该结构体对外暴露的同名状态。
    pub icons: Vec<String>,
}

static NAMED_ICON_CATALOG: Lazy<Vec<NamedIconFamily>> = Lazy::new(|| {
    serde_json::from_str(super::named_icon_generated::NAMED_ICON_CATALOG_JSON).unwrap_or_default()
});

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

fn icon_name_candidates(name: &str) -> Vec<String> {
    let dashed = name.replace('_', "-");
    let underscored = name.replace('-', "_");
    let mut candidates = Vec::new();
    for candidate in [name.to_string(), dashed, underscored] {
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

fn phosphor_weight_bucket(weight: Option<&serde_json::Value>) -> &'static str {
    let value = weight
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
        .unwrap_or(400);
    match value {
        ..=200 => "thin",
        201..=350 => "light",
        351..=599 => "regular",
        _ => "bold",
    }
}

fn named_icon_svg_text(
    family: &str,
    name: &str,
    weight: Option<&serde_json::Value>,
) -> Option<&'static str> {
    let family = sanitize_icon_token(family)?;
    let name = sanitize_icon_token(name)?;

    for candidate in icon_name_candidates(&name) {
        let relative_path = format!("{family}/{candidate}.svg");
        if let Some(file) = NAMED_ICON_DIR.get_file(&relative_path) {
            return std::str::from_utf8(file.contents()).ok();
        }
    }

    if family == "phosphor" {
        let bucket = phosphor_weight_bucket(weight);
        for candidate in icon_name_candidates(&name) {
            let filename = match bucket {
                "thin" => format!("{candidate}-thin.svg"),
                "light" => format!("{candidate}-light.svg"),
                "bold" => format!("{candidate}-bold.svg"),
                _ => format!("{candidate}.svg"),
            };
            let relative_path = format!("{family}/{bucket}/{filename}");
            if let Some(file) = NAMED_ICON_DIR.get_file(&relative_path) {
                return std::str::from_utf8(file.contents()).ok();
            }
        }
    }
    None
}

/// 执行 canonical_named_icon_family 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn canonical_named_icon_family(family: &str) -> Option<String> {
    sanitize_icon_token(family)
}

/// 执行 named_icon_family_label 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn named_icon_family_label(family: &str) -> String {
    sanitize_icon_token(family)
        .unwrap_or_else(|| family.trim().to_ascii_lowercase())
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn svg_color_value(color: Color) -> String {
    let r = (color.r * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (color.g * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (color.b * 255.0).round().clamp(0.0, 255.0) as u8;
    if (color.a - 1.0).abs() <= f32::EPSILON {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("rgba({r}, {g}, {b}, {:.3})", color.a.clamp(0.0, 1.0))
    }
}

fn colorize_icon_svg(svg_text: &str, color: Color) -> String {
    let color_value = svg_color_value(color);
    if let Some(index) = svg_text.find("<svg") {
        let insert_at = index + 4;
        let mut output = String::with_capacity(svg_text.len() + color_value.len() + 24);
        output.push_str(&svg_text[..insert_at]);
        output.push_str(&format!(r#" color="{color_value}""#));
        output.push_str(&svg_text[insert_at..]);
        output
    } else {
        svg_text.to_string()
    }
}

fn rasterize_svg_to_image_handle(svg_data: &str) -> Option<image::Handle> {
    use resvg::usvg::{self};
    use tiny_skia::Transform;

    #[cfg(not(target_arch = "wasm32"))]
    let mut opt = usvg::Options::default();
    #[cfg(target_arch = "wasm32")]
    let opt = usvg::Options::default();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();
        opt.fontdb = std::sync::Arc::new(fontdb);
    }

    let tree = usvg::Tree::from_str(svg_data, &opt).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
    let mut pm = pixmap.as_mut();
    resvg::render(&tree, Transform::default(), &mut pm);
    let png = pixmap.encode_png().ok()?;
    Some(image::Handle::from_bytes(png))
}

/// 执行 get_named_icon_image 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn get_named_icon_image(family: &str, name: &str, color: Color) -> Option<image::Handle> {
    get_named_icon_image_with_weight(family, name, None, color)
}

/// 执行 get_named_icon_image_with_weight 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn get_named_icon_image_with_weight(
    family: &str,
    name: &str,
    weight: Option<&serde_json::Value>,
    color: Color,
) -> Option<image::Handle> {
    let family_key = sanitize_icon_token(family)?;
    let name_key = sanitize_icon_token(name)?;
    let weight_key = weight
        .and_then(|value| {
            value.as_i64().map(|v| v.to_string()).or_else(|| value.as_str().map(str::to_string))
        })
        .unwrap_or_else(|| "default".to_string());
    let cache_key = format!(
        "{family_key}/{name_key}/{weight_key}/{:.4}/{:.4}/{:.4}/{:.4}",
        color.r, color.g, color.b, color.a
    );
    if let Some(handle) = NAMED_ICON_IMAGES.lock().get(&cache_key).cloned() {
        return Some(handle);
    }

    let svg_text = named_icon_svg_text(&family_key, &name_key, weight)?;
    let handle = rasterize_svg_to_image_handle(&colorize_icon_svg(svg_text, color))?;
    NAMED_ICON_IMAGES.lock().insert(cache_key, handle.clone());
    Some(handle)
}

/// 执行 named_icon_catalog 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn named_icon_catalog() -> &'static [NamedIconFamily] {
    NAMED_ICON_CATALOG.as_slice()
}

/// 执行 named_icon_family_json 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn named_icon_family_json(family: &str) -> Option<&'static str> {
    let family = canonical_named_icon_family(family)?;
    NAMED_ICON_FAMILY_JSONS.get(family.as_str()).copied()
}
#[cfg(test)]
#[path = "named_icons_tests.rs"]
mod named_icons_tests;
