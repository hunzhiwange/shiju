//! Render workflow dialogs and exercise their existing messages.

use super::*;
use crate::app::i18n::I18n;
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Rectangle, Renderer, Size, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, events: &[Event]) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        Point::ORIGIN,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    point: Point,
) -> Vec<u8> {
    assert!(
        send(
            ui,
            renderer,
            point,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now(),))]
        )
        .is_empty()
    );
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_38_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default, Debug)]
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Vector)>,
}

impl Operation for Bounds {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }

    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }

    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scrolls.push((bounds, offset));
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

fn modal_size(width: u32) -> Size<u32> {
    Size::new(width, if width == 420 { 480 } else { 720 })
}

#[tokio::test]
async fn ui_refresh_38_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let mut state = WorkflowState::default();
    state.video_edit.path = format!("/测试/繁體中文/{}/fixture.mp4", "video-path/".repeat(14));
    state.video_edit.editor = Some(text_editor::Content::with_text(
        "app:\n  name: 通用云端视频剪辑\nworkflow:\n  video_edit: {max_parallel: 5}",
    ));
    state.video_edit.busy = true;
    state.video_edit.restore_confirm = true;
    state.video_edit.error = Some("测试模板读取失败".into());

    let size = modal_size(420);
    let mut ui = scene(video_edit::modal(&state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "video-busy-420", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    assert!(geometry.inputs.iter().any(|(value, _)| value == &state.video_edit.path));
    let mut footer = Vec::new();
    for label in ["关闭", "丢弃草稿并重读", "恢复默认…", "保存模板", "在原生会话剪辑"]
    {
        let target = position(&geometry.labels, label);
        assert!(
            target.width >= label.chars().count() as f32 * 12.0
                && target.x >= 7.0
                && target.x + target.width <= size.width as f32 - 7.0
                && target.y > geometry.scrolls[0].0.y + geometry.scrolls[0].0.height,
            "{label} is clipped or outside the fixed footer: {target:?}"
        );
        footer.push(target.center());
    }
    for disabled in &footer[1..] {
        assert!(click(&mut ui, &mut renderer, *disabled).is_empty());
    }
    assert!(matches!(
        click(&mut ui, &mut renderer, footer[0]).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::CloseVideoTemplate)]
    ));
}

#[tokio::test]
async fn ui_refresh_38_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut state = WorkflowState::default();
    state.open_create_editor();
    let draft = state.app_editor.as_mut().unwrap();
    draft.name = "客服分流".into();
    draft.icon = "🤖".into();
    draft.description = "虛構的工作流程".into();
    draft.max_active_requests_input = "42".into();
    for (width, locale, theme, name) in [
        (420, DesktopLocale::SimplifiedChinese, Theme::Light, "app-editor-420"),
        (640, DesktopLocale::TraditionalChinese, Theme::Dark, "app-editor-640"),
        (960, DesktopLocale::English, Theme::Light, "app-editor-960"),
    ] {
        let i18n = I18n::new(locale);
        let size = modal_size(width);
        let mut ui = scene(build_app_editor_modal(i18n, &state), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, name, Point::ORIGIN);
        let geometry = bounds(&mut ui, &renderer);
        let field = position(&geometry.inputs, "客服分流").center();
        let save = position(&geometry.labels, i18n.__("保存应用信息")).center();
        let cancel = position(&geometry.labels, i18n.__("取消")).center();
        let scroll = geometry.scrolls[0].0;
        assert!(save.y > scroll.y + scroll.height && save.y < size.height as f32);
        assert!(click(&mut ui, &mut renderer, field).is_empty());
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
        assert!(matches!(
            key(&mut ui, &mut renderer, Key::Character("A".into()), Some("A")).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::AppEditorNameChanged(value))]
                if value == "客服分流A"
        ));
        for named in [Named::Enter, Named::Escape] {
            assert!(key(&mut ui, &mut renderer, Key::Named(named), None).is_empty());
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, cancel).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::CloseAppEditor)]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, save).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::SubmitAppEditor)]
        ));
    }
}

#[tokio::test]
async fn ui_refresh_38_empty_app_list_keeps_search_and_refresh() {
    let mut renderer = renderer().await;
    let size = modal_size(640);
    let i18n = I18n::new(DesktopLocale::English);
    let state = WorkflowState { saved_app_search_query: "fixture".into(), ..Default::default() };
    let mut ui = scene(build_saved_apps_view(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "app-list-empty-640", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let search = position(&geometry.inputs, "fixture");
    let refresh = position(&geometry.labels, i18n.__("刷新"));
    assert!((search.center_y() - refresh.center_y()).abs() < 0.5);
    assert!(click(&mut ui, &mut renderer, search.center()).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("s".into()), Some("s")).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::SavedAppSearchChanged(value))] if value == "fixtures"
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, refresh.center()).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::LoadSavedApps)]
    ));
}

#[tokio::test]
async fn ui_refresh_38_run_prompt_preserves_values_and_messages() {
    use state::{WorkflowRunInputDraft, WorkflowRunPromptDraft};

    let mut renderer = renderer().await;
    let size = modal_size(420);
    let i18n = I18n::new(DesktopLocale::TraditionalChinese);
    let mut state = WorkflowState {
        saved_app_run_prompt: Some(WorkflowRunPromptDraft {
            uuid: "local-fixture-workflow".into(),
            query: "示例任務".into(),
            inputs: vec![
                WorkflowRunInputDraft {
                    name: "internal".into(),
                    hidden: true,
                    ..Default::default()
                },
                WorkflowRunInputDraft {
                    name: "format".into(),
                    label: "輸出格式".into(),
                    input_type: "select".into(),
                    options: std::iter::once("標準".into())
                        .chain(
                            (1..30).map(|i| format!("格式 {i:02} / {}", "long-label-".repeat(10))),
                        )
                        .collect(),
                    value: "標準".into(),
                    ..Default::default()
                },
                WorkflowRunInputDraft {
                    name: "limit".into(),
                    label: "數量".into(),
                    input_type: "number".into(),
                    value: "invalid-number".into(),
                    required: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut ui =
        scene(build_saved_app_run_prompt_dialog(i18n, &state).unwrap(), &mut renderer, size);
    let geometry = bounds(&mut ui, &renderer);
    let label = position(&geometry.labels, "輸出格式");
    let select = Point::new(label.x + 35.0, label.y + label.height + 25.0);
    let closed = capture(&mut ui, &mut renderer, &Theme::Dark, size, "run-prompt-420", select);
    assert!(click(&mut ui, &mut renderer, select).is_empty());
    let outside = Point::new(1.0, 1.0);
    let opened = capture(&mut ui, &mut renderer, &Theme::Dark, size, "run-select-open", outside);
    assert!(opened != closed, "opening the select must draw the menu");
    let open_geometry = bounds(&mut ui, &renderer);
    let menu = open_geometry.scrolls[1].0;
    assert!(menu.x >= 0.0 && menu.y >= 0.0);
    assert!(menu.x + menu.width <= size.width as f32 && menu.y + menu.height <= size.height as f32);
    let row_pixel = |pixels: &[u8], row: usize| -> [u8; 4] {
        let x = (menu.x + 12.0) as usize;
        let y = (menu.y + 18.0 + row as f32 * 38.0) as usize;
        let start = (y * size.width as usize + x) * 4;
        pixels[start..start + 4].try_into().unwrap()
    };
    let mut hover_colors = Vec::new();
    for (row, name) in ["run-select-hover-one", "run-select-hover-two"].into_iter().enumerate() {
        let point = Point::new(menu.center_x(), menu.y + 18.0 + row as f32 * 38.0);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                point,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        let pixels = capture(&mut ui, &mut renderer, &Theme::Dark, size, name, point);
        hover_colors.push([row_pixel(&pixels, 0), row_pixel(&pixels, 1)]);
    }
    assert!(
        send(
            &mut ui,
            &mut renderer,
            outside,
            &[Event::Mouse(mouse::Event::CursorMoved { position: outside })]
        )
        .is_empty()
    );
    let cleared =
        capture(&mut ui, &mut renderer, &Theme::Dark, size, "run-select-outside", outside);
    for row in 0..2 {
        let resting = row_pixel(&cleared, row);
        assert_ne!(hover_colors[row][row], resting, "row {row} must highlight on hover");
        assert_eq!(hover_colors[1 - row][row], resting, "row {row} must clear after exit");
    }
    assert!(
        send(
            &mut ui,
            &mut renderer,
            menu.center(),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -100.0 },
            })]
        )
        .is_empty()
    );
    let scrolled = bounds(&mut ui, &renderer);
    assert_eq!(scrolled.scrolls[0], open_geometry.scrolls[0]);
    assert!(scrolled.scrolls[1].1.y > 0.0);
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::ArrowDown), None).is_empty());
    let messages = key(&mut ui, &mut renderer, Key::Named(Named::Enter), None);
    assert!(matches!(
        messages.as_slice(),
        [Message::WorkflowTool(WorkflowMessage::SavedAppRunInputChanged(1, value))]
            if value == &state.saved_app_run_prompt.as_ref().unwrap().inputs[1].options[1]
    ));
    let cancel = position(&geometry.labels, i18n.__("取消")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::CancelSavedAppRunPrompt)]
    ));
    let run = position(&geometry.labels, i18n.__("运行")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, run).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::SubmitSavedAppRunPrompt)]
    ));
    assert!(geometry.inputs.iter().any(|(value, _)| value == "invalid-number"));
    drop(ui);

    state.set_saved_app_run_prompt_error("數量 必须是数字");
    let mut failed =
        scene(build_saved_app_run_prompt_dialog(i18n, &state).unwrap(), &mut renderer, size);
    capture(&mut failed, &mut renderer, &Theme::Dark, size, "run-prompt-invalid", Point::ORIGIN);
    assert!(
        bounds(&mut failed, &renderer).labels.iter().any(|(label, _)| label == "數量 必须是数字")
    );
    drop(failed);

    let prompt = state.saved_app_run_prompt.as_mut().unwrap();
    prompt.loading = true;
    prompt.inputs.clear();
    prompt.error = None;
    let mut loading =
        scene(build_saved_app_run_prompt_dialog(i18n, &state).unwrap(), &mut renderer, size);
    capture(&mut loading, &mut renderer, &Theme::Light, size, "run-prompt-loading", Point::ORIGIN);
    assert!(bounds(&mut loading, &renderer).inputs.is_empty());
}

#[tokio::test]
async fn ui_refresh_38_preview_keeps_title_and_retry_fixed_when_body_scrolls() {
    use state::{
        WorkflowRunDetailDialog, WorkflowRunErrorPreview, WorkflowRunNodePreview,
        WorkflowRunPreview, WorkflowRunStatus,
    };

    let mut renderer = renderer().await;
    let size = modal_size(420);
    let i18n = I18n::new(DesktopLocale::English);
    let mut preview = WorkflowRunPreview {
        uuid: "fixture-workflow-id".into(),
        run_id: Some("fixture-run-id".into()),
        status: WorkflowRunStatus::Failed,
        nodes: (0..18)
            .map(|i| WorkflowRunNodePreview {
                id: format!("fixture-node-{i}"),
                title: format!("處理節點 {i} with a long mixed-language title"),
                status: Some("failed".into()),
                data: serde_json::json!({"elapsed_ms": 42}),
            })
            .collect(),
        text_output: "測試結果。 Fixture result.\n".repeat(24),
        error: Some(WorkflowRunErrorPreview {
            status: 500,
            code: "fixture_error".into(),
            message: "虛構錯誤".into(),
        }),
        is_finished: true,
    };
    let mut ui =
        scene(build_saved_app_run_preview_layer(i18n, &preview, false), &mut renderer, size);
    let before = bounds(&mut ui, &renderer);
    let title = position(&before.labels, i18n.__("运行预览"));
    let retry = position(&before.labels, i18n.__("重新执行"));
    let scroll = before.scrolls[0].0;
    assert!(title.y + title.height < scroll.y);
    assert!(retry.y > scroll.y + scroll.height && retry.y < size.height as f32);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "run-preview-before-scroll",
        Point::ORIGIN,
    );
    assert!(
        send(
            &mut ui,
            &mut renderer,
            scroll.center(),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1700.0 },
            })]
        )
        .is_empty()
    );
    let after = bounds(&mut ui, &renderer);
    assert!(after.scrolls[0].1.y > 0.0);
    assert_eq!(position(&after.labels, i18n.__("运行预览")), title);
    assert_eq!(position(&after.labels, i18n.__("重新执行")), retry);
    capture(
        &mut ui,
        &mut renderer,
        &Theme::Light,
        size,
        "run-preview-after-scroll",
        scroll.center(),
    );
    assert!(matches!(
        click(&mut ui, &mut renderer, retry.center()).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::RetrySavedAppRunPreview)]
    ));
    drop(ui);

    let detail = WorkflowRunDetailDialog {
        title: "輸出與調試資訊".into(),
        content: text_editor::Content::with_text(&preview.text_output),
        ..Default::default()
    };
    let mut detail_ui = scene(build_saved_app_run_detail_layer(i18n, &detail), &mut renderer, size);
    capture(&mut detail_ui, &mut renderer, &Theme::Dark, size, "run-detail-420", Point::ORIGIN);
    assert!(
        bounds(&mut detail_ui, &renderer).labels.iter().any(|(label, _)| label == &detail.title)
    );

    preview.nodes.clear();
    preview.text_output.clear();
    preview.error = None;
    preview.status = WorkflowRunStatus::Running;
    preview.is_finished = false;
    let size = modal_size(960);
    let mut running =
        scene(build_saved_app_run_preview_layer(i18n, &preview, false), &mut renderer, size);
    capture(&mut running, &mut renderer, &Theme::Dark, size, "run-preview-running", Point::ORIGIN);
    let cancel = position(&bounds(&mut running, &renderer).labels, i18n.__("取消运行"));
    assert!(matches!(
        click(&mut running, &mut renderer, cancel.center()).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::CancelSavedAppRun)]
    ));
}

#[tokio::test]
async fn ui_refresh_38_start_variable_keeps_invalid_draft_and_fixed_actions() {
    let mut renderer = renderer().await;
    let loaded = crate::apps::workflow::model::load_document_from_value(
        None,
        serde_yaml::from_str(
            r#"
app:
  name: Fixture workflow
workflow:
  graph:
    nodes:
      - id: start
        position: {x: 100, y: 100}
        data:
          type: start
          title: "起始變數"
          variables: []
    edges: []
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let mut state = WorkflowState::default();
    state.apply_loaded(loaded, (960.0, 520.0));
    state.open_edit_node_editor(Some("start")).unwrap();
    state.open_node_editor_start_variable_create();
    let editor = state.node_editor.as_mut().unwrap().start_variable_editor.as_mut().unwrap();
    editor.variable.label.clear();
    let i18n = I18n::new(DesktopLocale::TraditionalChinese);
    let size = modal_size(420);
    let mut ui = scene(build_start_variable_editor_modal(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "start-variable-420", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let save = position(&geometry.labels, i18n.__("保存")).center();
    let cancel = geometry
        .labels
        .iter()
        .rev()
        .find(|(label, _)| label == i18n.__("取消"))
        .unwrap()
        .1
        .center();
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStartCloseVariableEditor)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStartSubmitVariableEditor)]
    ));
    drop(ui);
    let error = state.submit_node_editor_start_variable_editor().unwrap_err();
    assert_eq!(&*error, "显示名称不能为空");
    assert!(state.node_editor.as_ref().unwrap().start_variable_editor.is_some());
}

fn check_workflow_help(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    title: &str,
    description: &str,
    artifact: &str,
) -> Point {
    let labels = bounds(ui, renderer).labels;
    assert!(
        labels.iter().all(|(label, _)| label != description),
        "{title}: static explanation still visible inline: {description}"
    );
    let label = position(&labels, title);
    let icon = Point::new(label.x + label.width + 15.0, label.center_y());
    assert!(Rectangle::with_size(Size::new(size.width as f32, size.height as f32)).contains(icon));
    let idle = capture(ui, renderer, theme, size, &format!("{artifact}-idle"), Point::ORIGIN);
    assert!(
        send(ui, renderer, icon, &[Event::Mouse(mouse::Event::CursorMoved { position: icon })])
            .is_empty()
    );
    let hovered = capture(ui, renderer, theme, size, &format!("{artifact}-hover"), icon);
    assert!(idle != hovered, "{title}: hover must open the help card");
    assert!(click(ui, renderer, icon).is_empty());
    let pinned = capture(ui, renderer, theme, size, &format!("{artifact}-open"), Point::ORIGIN);
    assert!(idle != pinned, "{title}: click must pin the help card");
    assert!(key(ui, renderer, Key::Named(Named::Escape), None).is_empty());
    let closed = capture(ui, renderer, theme, size, &format!("{artifact}-closed"), Point::ORIGIN);
    assert!(idle == closed, "{title}: Escape must close only the help card");
    icon
}

#[tokio::test]
async fn help_rollout_25_renders_contextual_help() {
    let mut renderer = renderer().await;
    let mut state = WorkflowState::default();
    state.open_create_editor();
    let draft = state.app_editor.as_mut().unwrap();
    draft.name = "Fixture app".into();
    draft.description = "Fixture draft text".into();
    draft.max_active_requests_input = "42".into();
    for (locale, theme, width) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, 420),
        (DesktopLocale::English, Theme::Light, 420),
        (DesktopLocale::TraditionalChinese, Theme::Dark, 640),
    ] {
        let i18n = I18n::new(locale);
        let size = Size::new(width, 680);
        for (mode, title, intro) in [
            (
                state::WorkflowAppEditorMode::Create,
                "新增应用",
                "创建一个新的 workflow 应用，后续可以单独保存成 yml 文件。",
            ),
            (
                state::WorkflowAppEditorMode::Edit("fixture-app".into()),
                "编辑应用信息",
                "修改应用元数据，不会丢失原始 Dify yml 里的节点业务字段。",
            ),
        ] {
            state.app_editor.as_mut().unwrap().mode = mode;
            let mut ui = scene(build_app_editor_modal(i18n, &state), &mut renderer, size);
            for (label, description, name) in [
                (title, intro, "title"),
                ("应用基础信息", "表单结构参考 Dify 应用设置页，先整理名称、图标和描述。", "group"),
                (
                    "使用 web app 图标替换 🤖",
                    "在分享和 Explore 场景里用应用图标替换默认机器人图标",
                    "icon",
                ),
            ] {
                check_workflow_help(
                    &mut ui,
                    &mut renderer,
                    &theme,
                    size,
                    i18n.__(label),
                    i18n.__(description),
                    &format!("app-{locale:?}-{title}-{name}"),
                );
            }
            let geometry = bounds(&mut ui, &renderer);
            assert!(position(&geometry.inputs, "42").y < size.height as f32);
            let icon_field = position(&geometry.labels, i18n.__("使用 web app 图标替换 🤖"));
            let toggle = Point::new(icon_field.x + 22.0, icon_field.center_y() + 28.0);
            assert!(matches!(
                click(&mut ui, &mut renderer, toggle).as_slice(),
                [Message::WorkflowTool(WorkflowMessage::AppEditorUseIconAsAnswerIconChanged(true))]
            ));
            let input = position(&geometry.inputs, "Fixture app").center();
            assert!(click(&mut ui, &mut renderer, input).is_empty());
            assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
            assert!(matches!(
                key(&mut ui, &mut renderer, Key::Character("A".into()), Some("A")).as_slice(),
                [Message::WorkflowTool(WorkflowMessage::AppEditorNameChanged(value))]
                    if value == "Fixture appA"
            ));
            assert!(matches!(
                click(&mut ui, &mut renderer, position(&geometry.labels, i18n.__("取消")).center())
                    .as_slice(),
                [Message::WorkflowTool(WorkflowMessage::CloseAppEditor)]
            ));
            assert!(matches!(
                click(
                    &mut ui,
                    &mut renderer,
                    position(&geometry.labels, i18n.__("保存应用信息")).center()
                )
                .as_slice(),
                [Message::WorkflowTool(WorkflowMessage::SubmitAppEditor)]
            ));
            if title == "编辑应用信息" {
                assert!(
                    click(
                        &mut ui,
                        &mut renderer,
                        position(&geometry.labels, i18n.__("未生成")).center()
                    )
                    .is_empty()
                );
            }
        }
    }
}

#[tokio::test]
async fn help_rollout_25_menu_uses_static_explanation() {
    let mut state = WorkflowState::default();
    state.open_create_editor();
    state.app_editor.as_mut().unwrap().name = "Private fixture value".into();
    let i18n = I18n::new(DesktopLocale::English);
    let theme = Theme::Light;
    let size = Size::new(960, 680);
    let mut renderer = renderer().await;
    let mut ui = scene(build_app_editor_modal(i18n, &state), &mut renderer, size);
    let title = i18n.__("新增应用");
    let description = i18n.__("创建一个新的 workflow 应用，后续可以单独保存成 yml 文件。");
    let icon =
        check_workflow_help(&mut ui, &mut renderer, &theme, size, title, description, "app-menu");
    assert!(click(&mut ui, &mut renderer, icon).is_empty());
    let card_text = Point::new(icon.x + 29.0, icon.y + 8.0);
    let line_height = iced::advanced::text::LineHeight::default().to_absolute(12.0.into());
    let item_height = f32::from(line_height) + 10.0;
    for index in [0, 3] {
        assert!(
            send(
                &mut ui,
                &mut renderer,
                card_text,
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))]
            )
            .is_empty()
        );
        let item =
            Point::new(card_text.x + 24.0, card_text.y + 3.0 + item_height * (index as f32 + 0.5));
        let messages = click(&mut ui, &mut renderer, item);
        match (index, messages.as_slice()) {
            (0, [Message::CopyCode(value)]) => assert_eq!(value, description),
            (3, [Message::View(crate::app::message::ViewMessage::OpenUrlExternal(url))]) => {
                assert_eq!(
                    url,
                    &format!(
                        "https://www.google.com/search?q={}",
                        urlencoding::encode(description)
                    )
                );
            }
            _ => panic!("unexpected help action: {messages:?}"),
        }
    }
}

#[tokio::test]
async fn help_rollout_25_renders_start_node_help_and_retains_status() {
    let mut renderer = renderer().await;
    for (locale, theme, mode) in [
        (DesktopLocale::SimplifiedChinese, Theme::Dark, "advanced-chat"),
        (DesktopLocale::English, Theme::Light, "workflow"),
        (DesktopLocale::TraditionalChinese, Theme::Dark, "advanced-chat"),
    ] {
        let mut state = WorkflowState::default();
        let loaded = crate::apps::workflow::model::load_document_from_value(
            None,
            serde_yaml::from_str(&format!(
                r#"
app:
  name: Fixture workflow
  mode: {mode}
workflow:
  graph:
    nodes:
      - id: start
        position: {{x: 100, y: 100}}
        data:
          type: start
          title: Fixture start
          variables: []
    edges: []
"#,
            ))
            .unwrap(),
        )
        .unwrap();
        state.apply_loaded(loaded, (800.0, 600.0));
        state.open_edit_node_editor(Some("start")).unwrap();
        let i18n = I18n::new(locale);
        let size = Size::new(800, 600);
        let mut ui = scene(build_node_editor_modal(i18n, &state), &mut renderer, size);
        if mode == "advanced-chat" {
            check_workflow_help(
                &mut ui,
                &mut renderer,
                &theme,
                size,
                "userinput.query",
                i18n.__("用户当前这一轮输入的问题文本。"),
                &format!("start-{locale:?}-query"),
            );
        }
        check_workflow_help(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            "userinput.files",
            i18n.__("用户上传的文件列表，可作为文件型输入或后续节点的附件来源。"),
            &format!("start-{locale:?}-files"),
        );
        let geometry = bounds(&mut ui, &renderer);
        position(&geometry.labels, "Array[File]");
        position(&geometry.labels, i18n.__("当前没有输入变量。新增后会写入 start.variables。"));
        if mode == "workflow" {
            position(&geometry.labels, i18n.__("兼容字段"));
            assert!(geometry.labels.iter().all(|(value, _)| value != "userinput.query"));
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, position(&geometry.labels, i18n.__("新增变量")).center())
                .as_slice(),
            [Message::WorkflowTool(WorkflowMessage::NodeEditorStartAddVariable)]
        ));
        assert!(matches!(
            click(&mut ui, &mut renderer, position(&geometry.labels, i18n.__("保存")).center())
                .as_slice(),
            [Message::WorkflowTool(WorkflowMessage::SubmitNodeEditor)]
        ));
    }
}
