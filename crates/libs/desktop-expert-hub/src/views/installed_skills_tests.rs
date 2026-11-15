//! 已安装技能列表渲染测试，验证真实条目数据可构建对应卡片组件。
//! 夹具覆盖安装来源与版本信息，守护列表到详情入口的基本展示契约。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use gateway_desktop_settings::DesktopSkillCatalogEntryDto;

use super::installed_skills::delete_confirmation;
use desktop_skill_hub::skill_hub::{ExpertHubSection, LoadState};
use super::{Message, ViewInput};

fn item(
    id: &str,
    source: &str,
    package_type: Option<&str>,
    source_pack: Option<&str>,
) -> DesktopSkillCatalogEntryDto {
    DesktopSkillCatalogEntryDto {
        id: id.to_string(),
        title: format!("中文技能 {id}"),
        description: "用于验证来源徽标、套件徽标和中文确认层".to_string(),
        kind: "personal".to_string(),
        resource_count: 1,
        installed: true,
        enabled: true,
        source: source.to_string(),
        source_path: Some(format!("/project/{id}")),
        origin: Some("workspace".to_string()),
        namespace: Some("public".to_string()),
        package_type: package_type.map(str::to_string),
        source_pack: source_pack.map(str::to_string),
    }
}

async fn assert_draws(app: &ViewInput<'_>, theme: Theme) {
    let mut element = { let (close, card) = delete_confirmation(app).expect("confirmation"); desktop_ui_controls::system_settings_common::settings_modal_overlay(Some(view(app)), close, card) };
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(960.0, 720.0);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds),
    );
    let viewport = Rectangle::with_size(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(960, 720), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[tokio::test]
async fn installed_skills_render_local_provider_and_suite_cards_with_confirmation() {
    let mut state = desktop_skill_hub::skill_hub::ExpertHubState::default();
    state.section = ExpertHubSection::Skills;
    state.public_market.installed_open = true;
    state.public_market.installed = LoadState::Loaded(vec![
        item("local", "workspace", None, None),
        item("market", "skillhub", Some("skill"), Some("safe-suite")),
        item("suite", "skillhub", Some("meta-skill"), Some("safe-suite")),
    ]);
    state.public_market.select_all_installed();
    state.public_market.delete_confirmation_open = true;

    for theme in [Theme::Light, Theme::Dark] {
        let app = ViewInput { expert_hub: &state, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: &theme };
        assert_draws(&app, theme.clone()).await;
    }
    let theme = Theme::Light;
    let app = ViewInput { expert_hub: &state, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: &theme };
    let _: iced::Element<'_, Message> = view(&app);
}

fn view<'a>(app: &ViewInput<'a>) -> iced::Element<'a, Message> {
 let i18n = desktop_i18n::I18n::new(app.desktop_locale);
 let header = super::installed_skills::header(i18n, move || desktop_ui_controls::system_settings_common::settings_hover_help_label(iced::widget::text(i18n.__("我安装的技能")).size(25), iced::widget::text(i18n.__("管理当前目录范围内可启用、关闭或卸载的本地技能")).into()), Message::SkillHub(crate::message::skill_hub::SkillHubMessage::Public(crate::message::public::PublicSkillMessage::CloseInstalled)));
 super::installed_skills::view(app, header, |message| message)
}
