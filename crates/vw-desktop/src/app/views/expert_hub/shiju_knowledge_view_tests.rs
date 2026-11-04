//! 从真实控件树读取文案并点击分页按钮，验证消息与状态边界。
use crate::app::message::skill_hub::knowledge::KnowledgeMessage;
use crate::app::{
    App, Message,
    account::SecretSkey,
    skill_hub::{ExpertHubSection, LoadState},
};
use iced::advanced::{
    Layout, Shell, clipboard, layout, mouse, renderer,
    renderer::Headless,
    widget::{Operation, Tree},
};
use iced::{Color, Element, Event, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{ShijuKnowledgeBaseDto, ShijuKnowledgeDocumentDto};

fn app() -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Knowledge;
    let state = &mut app.expert_hub.knowledge;
    state.sync_account("fixture".into(), SecretSkey::new("fixture-only").unwrap());
    state.bases.total = 45;
    state.bases.status = LoadState::Loaded(());
    state.bases.items = (0..20)
        .map(|i| ShijuKnowledgeBaseDto {
            kb_zid: format!("kb-{i}"),
            name: format!("超长中文知识库名称用于窄窗口渲染 {i}"),
            description: "知识库中文说明".repeat(5),
            status: "active".into(),
        })
        .collect();
    state.selected_base = Some("kb-0".into());
    state.documents.total = 65;
    state.documents.status = LoadState::Loaded(());
    state.documents.items = (0..20)
        .map(|i| ShijuKnowledgeDocumentDto {
            kb_zid: "kb-0".into(),
            doc_zid: format!("doc-{i}"),
            title: format!("中文文档 {i}"),
            original_filename: format!("中文原文件名 {i}.pdf"),
            lifecycle_status: "processing".into(),
        })
        .collect();
    app
}
#[derive(Default)]
struct Texts(Vec<(String, Rectangle)>);
impl Operation for Texts {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation)) {
        visit(self);
    }
    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.into(), bounds));
    }
}
pub(super) async fn inspect(
    mut element: Element<'_, Message>,
    theme: Theme,
    width: f32,
    click: Option<(&str, usize)>,
) -> (Vec<String>, Vec<Message>) {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    let bounds = Size::new(width, 800.0);
    let viewport = Rectangle::with_size(bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds),
    );
    let mut texts = Texts::default();
    element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut texts);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(width as u32, 800), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
    let mut messages = vec![];
    if let Some((label, occurrence)) = click {
        let rect = texts
            .0
            .iter()
            .filter(|(value, _)| value == label)
            .nth(occurrence)
            .expect("visible button")
            .1;
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            element.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                mouse::Cursor::Available(rect.center()),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &viewport,
            );
        }
    }
    (texts.0.into_iter().map(|(s, _)| s).collect(), messages)
}
fn knowledge(messages: &[Message]) -> &KnowledgeMessage {
    match &messages[0] {
        Message::SkillHub(crate::app::message::SkillHubMessage::Knowledge(msg)) => msg,
        _ => panic!("knowledge message"),
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_view_exposes_independent_paging() {
    let mut app = app();
    let (texts, messages) =
        inspect(super::view(&app), Theme::Light, 720.0, Some(("下一页", 1))).await;
    assert!(texts.iter().any(|s| s.contains("1 / 3") && s.contains("45")));
    assert!(texts.iter().any(|s| s.contains("1 / 4") && s.contains("65")));
    assert!(matches!(knowledge(&messages), KnowledgeMessage::DocumentsPage(2)));
    assert!(app.expert_hub.knowledge.documents.set_page(2));
    assert_eq!(app.expert_hub.knowledge.bases.query.page, 1);
    let (_, messages) = inspect(super::view(&app), Theme::Dark, 1100.0, Some(("下一页", 0))).await;
    assert!(matches!(knowledge(&messages), KnowledgeMessage::BasesPage(2)));
    assert!(app.expert_hub.knowledge.set_base_page(2));
    assert_eq!(app.expert_hub.knowledge.documents.query.page, 1);
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_view_disables_invalid_navigation() {
    let mut app = app();
    for (page, total, loading, label, disabled) in [
        (1, 45, false, "上一页", true),
        (2, 45, false, "上一页", false),
        (3, 45, false, "下一页", true),
        (2, 45, true, "下一页", true),
        (1, 0, false, "下一页", true),
        (1, 1, false, "下一页", true),
    ] {
        let state = &mut app.expert_hub.knowledge;
        state.bases.query.page = page;
        state.bases.total = total;
        state.bases.status =
            if loading { LoadState::Loading { generation: 1 } } else { LoadState::Loaded(()) };
        let (_, messages) = inspect(super::view(&app), Theme::Dark, 720.0, Some((label, 0))).await;
        assert_eq!(messages.is_empty(), disabled, "page={page}, total={total}, loading={loading}");
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_view_renders_login_empty_error_states() {
    let mut app = app();
    for (status, expected) in [
        (LoadState::NeedsLogin, "请先登录识矩账号"),
        (LoadState::Loading { generation: 1 }, "加载中…"),
        (LoadState::Loaded(()), "暂无知识库"),
        (LoadState::Error("fixture_error".into()), "加载失败，请重试"),
    ] {
        app.expert_hub.knowledge.bases.status = status;
        app.expert_hub.knowledge.bases.items.clear();
        for theme in [Theme::Light, Theme::Dark] {
            let (texts, _) = inspect(super::view(&app), theme, 720.0, None).await;
            assert!(texts.iter().any(|s| s == expected), "missing {expected}: {texts:?}");
        }
    }
    app.expert_hub.knowledge.bases.status = LoadState::Loaded(());
    app.expert_hub.knowledge.documents.items.clear();
    let (texts, _) = inspect(super::view(&app), Theme::Light, 720.0, None).await;
    assert!(texts.iter().any(|s| s == "暂无文档"));
}

#[tokio::test(flavor = "multi_thread")]
async fn shiju_knowledge_view_browses_all_server_pages_and_opens_documents() {
    use vw_gateway_client::ShijuKnowledgePageDto;
    let mut app = app();
    for page in 1..=3 {
        let state = &mut app.expert_hub.knowledge;
        assert!(state.set_base_page(page));
        let request = state.begin_bases().unwrap();
        let start = (page - 1) * 20;
        let items = (start..(start + 20).min(45))
            .map(|index| ShijuKnowledgeBaseDto {
                kb_zid: format!("kb-{index}"),
                name: format!("知识库 {index}"),
                description: "中文说明".into(),
                status: "active".into(),
            })
            .collect();
        assert_eq!(
            state.finish_bases(
                &request,
                Ok(ShijuKnowledgePageDto { items, page, page_size: 20, total: 45 })
            ),
            Some(false)
        );
        let (texts, _) = inspect(super::view(&app), Theme::Dark, 720.0, None).await;
        assert!(texts.contains(&format!("知识库 {start}")));
        assert!(texts.contains(&format!("{} / 3 · 总数 45", page)));
    }
    for page in 1..=4 {
        let state = &mut app.expert_hub.knowledge;
        state.documents.total = 65;
        assert!(state.documents.set_page(page));
        let request = state.begin_documents().unwrap();
        let kb = state.selected_base.clone().unwrap();
        let start = (page - 1) * 20;
        let items = (start..(start + 20).min(65))
            .map(|index| ShijuKnowledgeDocumentDto {
                kb_zid: kb.clone(),
                doc_zid: format!("doc-{index}"),
                title: format!("文档 {index}"),
                original_filename: format!("原文件 {index}.pdf"),
                lifecycle_status: "processing".into(),
            })
            .collect();
        assert_eq!(
            state.finish_documents(
                &request,
                Ok(ShijuKnowledgePageDto { items, page, page_size: 20, total: 65 })
            ),
            Some(false)
        );
        let (texts, messages) =
            inspect(super::view(&app), Theme::Light, 720.0, Some(("阅读", 0))).await;
        assert!(texts.contains(&format!("原文件 {start}.pdf")));
        assert!(texts.contains(&"状态：processing".into()));
        assert!(texts.contains(&format!("{} / 4 · 总数 65", page)));
        assert!(
            matches!(knowledge(&messages), KnowledgeMessage::OpenReader { kb: actual_kb, doc } if actual_kb == &kb && doc == &format!("doc-{start}"))
        );
        assert_eq!(app.expert_hub.knowledge.bases.query.page, 3);
    }
    // 服务端总数收缩，真实状态应纠正到新末页，界面显示新总数。
    let state = &mut app.expert_hub.knowledge;
    let request = state.begin_documents().unwrap();
    assert_eq!(
        state.finish_documents(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 4, page_size: 20, total: 21 })
        ),
        Some(true)
    );
    let request = state.begin_documents().unwrap();
    assert_eq!(
        state.finish_documents(
            &request,
            Ok(ShijuKnowledgePageDto { items: vec![], page: 2, page_size: 20, total: 21 })
        ),
        Some(false)
    );
    let (texts, messages) =
        inspect(super::view(&app), Theme::Dark, 720.0, Some(("下一页", 1))).await;
    assert!(texts.contains(&"2 / 2 · 总数 21".into()));
    assert!(messages.is_empty());
}

#[test]
fn shiju_knowledge_view_notifies_accepted_errors_only() {
    use crate::app::account::{AccountIdentity, AccountOrigin, AccountRepository, VerifiedAccount};
    use crate::app::message::skill_hub::knowledge::update;
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("fixture-only").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    let account = app.account.active_account().unwrap();
    app.expert_hub.knowledge.sync_account(account.id().to_string(), account.skill_hub_credential());
    let stale = app.expert_hub.knowledge.begin_bases().unwrap();
    let current = app.expert_hub.knowledge.begin_bases().unwrap();
    let _ = update(&mut app, KnowledgeMessage::BasesLoaded(stale, Err("obsolete".into())));
    assert!(app.active_toast.is_none());
    let _ = update(
        &mut app,
        KnowledgeMessage::BasesLoaded(current.clone(), Err("fixture-only-error".into())),
    );
    assert_eq!(app.active_toast.as_ref().unwrap().message, "资料加载失败，请重试");
    app.active_toast = None;
    let _ = update(&mut app, KnowledgeMessage::BasesLoaded(current, Err("duplicate".into())));
    assert!(app.active_toast.is_none());
}
