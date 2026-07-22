use super::Icon;
use iced::widget::svg;
use std::collections::HashMap;

pub(super) fn register_icons(m: &mut HashMap<Icon, svg::Handle>) {
    // === 文件类型图标 ===

    // Rust 语言文件图标
    m.insert(
        Icon::Rust,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/rust.svg")),
    );

    // TypeScript 语言文件图标
    m.insert(
        Icon::Typescript,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/typescript.svg"
        )),
    );

    // JavaScript 语言文件图标
    m.insert(
        Icon::Javascript,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/javascript.svg"
        )),
    );

    // JSON 文件图标
    m.insert(
        Icon::Json,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/json.svg")),
    );

    // TOML 配置文件图标
    m.insert(
        Icon::Toml,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/toml.svg")),
    );

    // YAML 配置文件图标
    m.insert(
        Icon::Yaml,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/yaml.svg")),
    );

    // Markdown 文件图标
    m.insert(
        Icon::Markdown,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/markdown.svg"
        )),
    );

    // HTML 文件图标
    m.insert(
        Icon::Html,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/html.svg")),
    );

    // CSS 样式文件图标
    m.insert(
        Icon::Css,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/css.svg")),
    );

    // Python 语言文件图标
    m.insert(
        Icon::Python,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/python.svg"
        )),
    );

    // Go 语言文件图标
    m.insert(
        Icon::Go,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/file-types/go.svg")),
    );

    // 控制台图标
    m.insert(
        Icon::Console,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/console.svg"
        )),
    );

    // 文档图标
    m.insert(
        Icon::Document,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/file-types/document.svg"
        )),
    );

    // 二维码图标
    m.insert(
        Icon::QrCode,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/qr-code.svg"
        )),
    );

    // 剪刀/剪切图标
    m.insert(
        Icon::Scissors,
        svg::Handle::from_memory(include_bytes!(
            "../../../../../assets/icons/bootstrap/scissors.svg"
        )),
    );

    // 复制图标
    m.insert(
        Icon::Copy,
        svg::Handle::from_memory(include_bytes!("../../../../../assets/icons/bootstrap/copy.svg")),
    );
}
#[cfg(test)]
#[path = "icon_files_tests.rs"]
mod icon_files_tests;
