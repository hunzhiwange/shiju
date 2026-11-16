// 宿主契约：模型、路由、分类及多模态四个领域的错误横幅按 active_tab 投影。
//! 验证模型设置的提供商筛选、批量操作、状态视图与详情覆盖层。
//! 搜索分别匹配提供商、模型编号和名称，禁用模型不得意外消失于详情流程。

use super::*;
use crate::app::state::{
    ModelConfigTab,
};
use crate::app::{App, Message};
use iced::advanced::widget::Tree;

// 提供商目录完全由 fixture 提供，避免网络刷新状态进入筛选和批量操作测试。
fn test_app() -> App {
    App::new().0
}

#[test]
fn model_settings_error_banner_is_local_to_tabbed_view() {
    let mut app = test_app();
    let cases = [
        (ModelConfigTab::Models, "models error"),
        (ModelConfigTab::ModelRoutes, "routes error"),
        (ModelConfigTab::QueryClassification, "classification error"),
        (ModelConfigTab::Multimodal, "multimodal error"),
    ];

    for (tab, _) in cases {
        app.model_settings.active_tab = tab;
        assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 2);
    }

    app.model_settings.save_error = Some((cases[0].1.to_string()).into());
    app.model_routes_settings.save_error = Some((cases[1].1.to_string()).into());
    app.query_classification_settings.save_error = Some(cases[2].1.to_string());
    app.multimodal_settings.save_error = Some(cases[3].1.to_string());

    for (tab, expected) in cases {
        app.model_settings.active_tab = tab;
        assert_eq!(active_tab_save_error(&app), Some(expected));
        assert_eq!(Tree::new(view(&app).as_widget()).children.len(), 3);
    }
}
