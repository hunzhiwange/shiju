//! 公共技能市场测试，覆盖来源选择、推荐卡片和三类工作助手标签。
//! 来源选择器仅在确有多个来源时出现，窄布局仍需保持卡片背景与中文文本可读。

use iced::advanced::renderer::Headless;
use iced::advanced::widget::operation::{Focusable, Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, mouse, renderer, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Color, Element, Event, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;
use vw_gateway_client::{
    SkillMarketCategoryDto, SkillMarketCoordinateDto, SkillMarketSkillDto,
    SkillMarketSkillLocatorDto, SkillMarketSkillSetDto, SkillMarketSkillSetPageDto,
    SkillMarketSort, SkillMarketSourceDto,
};

use super::public_skills::{grid_column_count, public_tab_specs, source_selector_visible, view};
use super::styles::{card_style, featured_card_style};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicSkillTab};
use crate::app::{App, Message};

fn source(source: &str) -> SkillMarketSourceDto {
    SkillMarketSourceDto { source: source.to_string(), display_name: source.to_string() }
}

fn skill() -> SkillMarketSkillDto {
    SkillMarketSkillDto {
        coordinate: SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: "skillhub".to_string(),
                namespace: "公开命名空间".to_string(),
                slug: "long-title".to_string(),
            },
            version: "1.0.0".to_string(),
        },
        name: "超长中文技能标题用于验证窄窗口渲染不会崩溃".to_string(),
        description: "这是一个包含中文的技能介绍，用于验证默认字体整形和暗黑主题显示。".repeat(4),
        author: "技能作者".to_string(),
        category: "开发工具".to_string(),
        tags: vec!["中文".to_string(), "自动化".to_string()],
        icon: None,
        downloads: 120,
        installs: 45,
        rating: 4.8,
        updated_at: "2026-07-30".to_string(),
    }
}

fn populated_app(tab: PublicSkillTab) -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Skills;
    app.expert_hub.public_market.tab = tab;
    app.expert_hub.public_market.sources =
        LoadState::Loaded(vec![source("skillhub"), source("second")]);
    app.expert_hub.public_market.current_source = Some("skillhub".to_string());
    app.expert_hub.public_market.recommended = LoadState::Loaded(vec![skill()]);
    app.expert_hub.public_market.market =
        LoadState::Loaded(vw_gateway_client::SkillMarketSkillPageDto {
            skills: vec![skill()],
            total: 1,
            page: 1,
            page_size: 24,
        });
    app.expert_hub.public_market.suites = LoadState::Loaded(SkillMarketSkillSetPageDto {
        skillsets: vec![SkillMarketSkillSetDto {
            source: "skillhub".to_string(),
            slug: "automation-suite".to_string(),
            name: "中文自动化测试套件".to_string(),
            description: "覆盖完整研发流程的技能套件".to_string(),
            author: "SkillHub".to_string(),
            icon: None,
            member_count: 6,
            updated_at: "2026-07-30".to_string(),
        }],
        total: 1,
        page: 1,
        page_size: 24,
    });
    app
}

async fn assert_draws(mut element: Element<'_, Message>, theme: &Theme, width: f32) {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(width, 800.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(width as u32, 800), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn public_skill_page_builds_three_workbuddy_tabs() {
    assert_eq!(
        public_tab_specs(),
        &[
            ("推荐", PublicSkillTab::Recommended),
            ("SkillHub", PublicSkillTab::SkillHub),
            ("套件", PublicSkillTab::Suites),
        ]
    );
    assert_eq!(grid_column_count(720.0), 2);
    assert_eq!(grid_column_count(1_440.0), 4);
}

#[test]
fn source_selector_appears_only_for_multiple_sources() {
    assert!(!source_selector_visible(&LoadState::Idle));
    assert!(!source_selector_visible(&LoadState::Loaded(vec![source("skillhub")])));
    assert!(source_selector_visible(&LoadState::Loaded(vec![
        source("skillhub"),
        source("second"),
    ])));
}

#[test]
fn featured_skill_cards_keep_the_recommended_card_background() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(featured_card_style(&theme).background, card_style(&theme).background);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn public_skill_page_renders_chinese_in_both_themes() {
    for tab in [PublicSkillTab::Recommended, PublicSkillTab::SkillHub, PublicSkillTab::Suites] {
        let app = populated_app(tab);
        for theme in [Theme::Light, Theme::Dark] {
            for width in [720.0, 1_440.0] {
                assert_draws(view(&app), &theme, width).await;
            }
        }
    }
}

pub(super) const CONTROL_LOCALES: [(DesktopLocale, &str); 3] = [
    (DesktopLocale::SimplifiedChinese, "虚构技能目录"),
    (DesktopLocale::TraditionalChinese, "虛構技能目錄"),
    (DesktopLocale::English, "Sample skill catalog"),
];

pub(super) async fn control_renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), Pixels(14.0), Some("tiny-skia"))
        .await
        .expect("headless renderer")
}

struct FocusBounds(Rectangle);

impl Operation for FocusBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if bounds == self.0 {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

#[derive(Default)]
struct MenuBounds(Option<Rectangle>);

impl Operation for MenuBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&iced::advanced::widget::Id::from("settings-select-menu")) {
            self.0 = Some(bounds);
        }
    }
}

pub(super) struct SkillControls<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> SkillControls<'a> {
    pub(super) fn new(mut view: Element<'a, Message>, renderer: &mut Renderer, width: u32) -> Self {
        let size = Size::new(width, 680);
        let bounds = Size::new(width as f32, size.height as f32);
        let mut tree = Tree::new(view.as_widget());
        let layout = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(view, bounds, Cache::new(), renderer);
        Self { ui, layout, size }
    }

    pub(super) fn bounds(&self, path: &[usize]) -> Rectangle {
        let bounds = path
            .iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds();
        assert!(bounds.width > 0.0 && bounds.x + bounds.width <= self.size.width as f32);
        bounds
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        event: Event,
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(&[event], cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    pub(super) fn enter(&mut self, renderer: &mut Renderer, bounds: Rectangle) -> Vec<Message> {
        self.ui.operate(renderer, &mut FocusBounds(bounds));
        self.key(renderer, Named::Enter)
    }

    pub(super) fn key(&mut self, renderer: &mut Renderer, key: Named) -> Vec<Message> {
        self.send(
            renderer,
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: Key::Named(key),
                key: Key::Named(key),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    pub(super) fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        let cursor = mouse::Cursor::Available(point);
        let mut messages = self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        messages.extend(self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        ));
        messages
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, cursor: mouse::Cursor) -> Vec<u8> {
        self.send(
            renderer,
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            cursor,
        );
        self.ui.draw(
            renderer,
            theme,
            &renderer::Style { text_color: theme.palette().text },
            cursor,
        );
        renderer.screenshot(self.size, 1.0, theme.palette().background)
    }

    fn menu_bounds(&mut self, renderer: &Renderer, name: &str) -> Rectangle {
        let mut menu = MenuBounds::default();
        self.ui.operate(renderer, &mut menu);
        menu.0.unwrap_or_else(|| panic!("{name} should open the settings menu on Enter"))
    }

    pub(super) fn choose_second(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        trigger: Rectangle,
        name: &str,
    ) -> Vec<Message> {
        let closed = self.draw(renderer, theme, mouse::Cursor::Unavailable);
        assert!(self.enter(renderer, trigger).is_empty());
        let opened = self.draw(renderer, theme, mouse::Cursor::Unavailable);
        let menu = self.menu_bounds(renderer, name);

        // Menu rows are 36px high with 2px between them.
        let first_y = (menu.y + 18.0) as usize;
        let second = Point::new(menu.center().x, menu.y + 56.0);
        let width = self.size.width as usize;
        let pixel = |pixels: &[u8], x: usize, y: usize| {
            let offset = (y * width + x) * 4;
            [pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3]]
        };
        let right = (menu.x + menu.width) as usize;
        let mark_x = right - 25;
        let region = |center_y: usize| {
            (center_y - 7..center_y + 7)
                .flat_map(move |y| (mark_x - 9..mark_x + 9).map(move |x| (x, y)))
        };
        let changed =
            region(first_y).filter(|&(x, y)| pixel(&closed, x, y) != pixel(&opened, x, y)).count();
        assert!(changed > 10, "{name} should draw an overlay on Enter");

        let cursor = mouse::Cursor::Available(second);
        self.send(renderer, Event::Mouse(mouse::Event::CursorMoved { position: second }), cursor);
        let hovered = self.draw(renderer, theme, cursor);
        let changed = region(second.y as usize)
            .filter(|&(x, y)| pixel(&opened, x, y) != pixel(&hovered, x, y))
            .count();
        assert!(changed > 10, "{name} should highlight the hovered option");

        let background = pixel(&hovered, mark_x, first_y - 9);
        let marks = region(first_y).filter(|&(x, y)| pixel(&hovered, x, y) != background).count();
        assert!(marks > 10, "{name} should draw a checkmark for the current option");

        self.click(renderer, second)
    }
}

#[tokio::test]
async fn skill_form_controls_render_source_and_sort() {
    let mut renderer = control_renderer().await;
    let mut app = populated_app(PublicSkillTab::SkillHub);
    app.expert_hub.public_market.query.keyword_draft = "fixture".into();
    for (locale, name) in CONTROL_LOCALES {
        app.desktop_locale = locale;
        // Identical long display names must still select the correct source ID.
        let mut sources = vec![
            SkillMarketSourceDto { source: "skillhub".into(), display_name: name.repeat(8) },
            SkillMarketSourceDto { source: "second".into(), display_name: name.repeat(8) },
        ];
        sources.extend((2..12).map(|index| source(&format!("fixture-{index}"))));
        app.expert_hub.public_market.sources = LoadState::Loaded(sources);
        app.expert_hub.public_market.categories = LoadState::Loaded(
            (1..=3)
                .map(|index| SkillMarketCategoryDto {
                    slug: format!("category-{index}"),
                    name: format!("{name} {index}"),
                    skill_count: index,
                })
                .collect(),
        );
        for theme in [Theme::Light, Theme::Dark] {
            for width in [420, 760] {
                let mut controls = SkillControls::new(view(&app), &mut renderer, width);
                let source = controls.bounds(&[0, 0, 1, 1]);
                let sort = controls.bounds(&[0, 1, 0, 1]);
                assert_eq!(source.width, 160.0);
                assert_eq!(sort.width, 130.0);
                assert!(matches!(
                    controls.choose_second(&mut renderer, &theme, source, "source").as_slice(),
                    [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SelectSource(id)))]
                        if id == "second"
                ));
                assert!(controls.enter(&mut renderer, source).is_empty());
                assert!(controls.key(&mut renderer, Named::End).is_empty());
                controls.draw(&mut renderer, &theme, mouse::Cursor::Unavailable);
                let menu = controls.menu_bounds(&renderer, "source");
                let last = Point::new(menu.center().x, menu.y + menu.height - 18.0);
                assert!(matches!(
                    controls.click(&mut renderer, last).as_slice(),
                    [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SelectSource(id)))]
                        if id == "fixture-11"
                ));
                assert!(matches!(
                    controls.choose_second(&mut renderer, &theme, sort, "sort").as_slice(),
                    [Message::SkillHub(SkillHubMessage::Public(PublicSkillMessage::SelectSort(
                        SkillMarketSort::Downloads
                    )))]
                ));
                let input = controls.bounds(&[0, 0, 2, 0]);
                let submit = controls.bounds(&[0, 0, 2, 1]);
                assert!(input.width >= 300.0);
                for messages in [
                    controls.enter(&mut renderer, input),
                    controls.click(&mut renderer, submit.center()),
                ] {
                    assert!(matches!(
                        messages.as_slice(),
                        [Message::SkillHub(SkillHubMessage::Public(
                            PublicSkillMessage::SubmitQuery
                        ))]
                    ));
                }
            }
        }
    }
}
