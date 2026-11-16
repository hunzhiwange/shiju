//! Model configuration page layout; sibling page content is supplied by the host.
use crate::state::ModelConfigTab;
use desktop_ui_controls::system_settings_common::{
    settings_error_banner, settings_tab_button, settings_tab_row,
};
use iced::{Element, Length, widget::column};
#[derive(Clone, Copy)]
pub struct ModelTabDescriptor {
    pub tab: ModelConfigTab,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
}
impl ModelTabDescriptor {
    pub const fn new(
        tab: ModelConfigTab,
        label: &'static str,
        keywords: &'static [&'static str],
    ) -> Self {
        Self { tab, label, keywords }
    }
}
pub const MODEL_TABS: &[ModelTabDescriptor] = &[
    ModelTabDescriptor::new(
        ModelConfigTab::Models,
        "模型",
        &[
            "models",
            "搜尋模型",
            "搜索模型",
            "模型详情",
            "提供商",
            "模型 id",
            "上下文窗口",
            "最大输出",
            "温度",
        ],
    ),
    ModelTabDescriptor::new(
        ModelConfigTab::ModelRoutes,
        "模型路由",
        &["model routes", "模型路由", "routing", "匹配模式", "pattern", "priority", "优先级"],
    ),
    ModelTabDescriptor::new(
        ModelConfigTab::QueryClassification,
        "模型分类",
        &["model classification", "模型分類", "classification", "rules", "查询分类", "规则"],
    ),
    ModelTabDescriptor::new(
        ModelConfigTab::Multimodal,
        "多模态",
        &[
            "multimodal",
            "image",
            "vision",
            "图片模型",
            "OCR",
            "文字提取",
            "远程抓取",
            "remote fetch",
        ],
    ),
];

pub fn view<'a, M: Clone + 'a>(
    active: ModelConfigTab,
    locale: config_ui::DesktopLocale,
    content: Element<'a, M>,
    error: Option<String>,
    selected: fn(ModelConfigTab) -> M,
) -> Element<'a, M> {
    let tabs = settings_tab_row(MODEL_TABS.iter().map(|descriptor| {
        settings_tab_button(
            desktop_i18n::settings_models::text(locale, descriptor.label),
            descriptor.tab == active,
            Some(selected(descriptor.tab)),
        )
        .into()
    }))
    .wrap();
    let mut sections = column![tabs].spacing(16).width(Length::Fill);
    if let Some(error) = error {
        sections = sections.push(settings_error_banner(error));
    }
    sections.push(content).into()
}
