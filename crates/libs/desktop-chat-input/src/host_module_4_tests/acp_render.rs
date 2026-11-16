use super::ui_driver::*;
use crate::input_panel::{acp_options, model_views, send_message};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_settings_providers::state::{ModelSettingsState, ModelSummary, ProviderModelsSummary};
use iced::keyboard::{Key, key::Named};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Point, Renderer, Size, Theme};
use vw_api_acp_model::{
    acp_model_source::{AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason},
    acp_session::{AcpAccessStatus, AcpAccessSupport, AcpChoice, AcpOptionKind, AcpSessionOption},
};
#[derive(Debug, Clone)]
enum Message {
    Acp(acp_options::Action),
    Send(send_message::Message),
    #[allow(dead_code)]
    Model(model_views::Action),
    Agent,
    #[allow(dead_code)]
    Help(crate::input_panel::task_mode::Message),
}
const CONFIG_ID: &str = "fixture/backend-option";
const NEXT_VALUE: &str = "fixture/backend-value-2";
struct Fixture {
    runtime: desktop_session::session::agent::SessionRuntimeState<()>,
    model_settings: ModelSettingsState,
    desktop_locale: DesktopLocale,
    app_theme: Theme,
    permission_context: bool,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            runtime: desktop_session::session::agent::SessionRuntimeState::new(),
            model_settings: Default::default(),
            desktop_locale: DesktopLocale::SimplifiedChinese,
            app_theme: Theme::Dark,
            permission_context: false,
        }
    }
}
impl Fixture {
    fn current_session_runtime_mut(
        &mut self,
    ) -> &mut desktop_session::session::agent::SessionRuntimeState<()> {
        &mut self.runtime
    }
}
fn fixture() -> Fixture {
    let mut app = Fixture::default();
    let runtime = app.current_session_runtime_mut();
    runtime.draft = Some(desktop_session::session::agent::SessionDraft {
        project_path: "/workspace".into(),
        directory: "/workspace".into(),
        history_len: 0,
    });
    runtime.acp_agent = Some("codex".into());
    runtime.acp_model_source = AcpModelSource::SessionModel;
    runtime.acp_options_state.options.connected = true;
    runtime.acp_options_state.options.model_source_capabilities = vec![
        AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel },
        AcpModelSourceCapability::Unsupported {
            source: AcpModelSource::CliLocal,
            reason: AcpModelSourceRejectionReason::MissingCredentials,
        },
    ];
    runtime.model = format!("fixture/{}", "long-path/".repeat(12));
    runtime.auto_model = false;
    runtime.task_mode_subtasks = vec!["多行任務\nSecond line".into(), "Fallback editor".into()];
    runtime.task_mode_subtask_editors =
        vec![iced::widget::text_editor::Content::with_text(&runtime.task_mode_subtasks[0])];
    runtime.acp_options_state.options.options = vec![AcpSessionOption {
        kind: AcpOptionKind::Config { id: CONFIG_ID.into() },
        name: "動態配置項目與較長的後端顯示名稱".into(),
        current: "fixture/backend-value-1".into(),
        choices: vec![
            AcpChoice { id: "fixture/backend-value-1".into(), name: "當前選項".into() },
            AcpChoice { id: NEXT_VALUE.into(), name: "待選選項".into() },
        ],
    }];
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "fixture".into(),
        name: "測試供應商".into(),
        models: (0..15)
            .map(|index| ModelSummary {
                id: format!("model-{index:02}"),
                name: if index == 13 {
                    "測試長名稱".repeat(10)
                } else {
                    format!("測試模型 {index:02}")
                },
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 4096,
                detail: serde_json::json!({}),
            })
            .collect(),
    }];
    app
}

impl Fixture {
    fn current_session_runtime(&self) -> &desktop_session::session::agent::SessionRuntimeState<()> {
        &self.runtime
    }
    fn started(&self) -> bool {
        desktop_session::session::runtime::current_session_has_started(
            Some(&self.runtime),
            true,
            false,
        )
    }
    fn state(&self) -> acp_options::OptionsState<'_> {
        let runtime = &self.runtime;
        acp_options::OptionsState {
            i18n: I18n::new(self.desktop_locale),
            acp_agent: runtime.acp_agent.as_deref(),
            acp_options: &runtime.acp_options_state.options,
            acp_model_source: runtime.acp_model_source,
            is_requesting: runtime.request_lifecycle.is_requesting,
            acp_options_pending: runtime.acp_options_state.pending,
            source_pending: runtime.acp_options_state.source_pending.is_some(),
            auto_model: runtime.auto_model,
            model: &runtime.model,
            full_access_enabled: runtime.full_access_enabled,
            acp_rebuild_required: runtime.acp_rebuild_required,
        }
    }
    fn source(&self) -> Element<'_, Message> {
        let help: Element<'_, crate::input_panel::task_mode::Message> =
            desktop_ui_controls::system_settings_common::help::settings_selectable_help_label(
                I18n::new(self.desktop_locale), &self.app_theme, "模型来源",
                desktop_ui_controls::system_settings_common::settings_field_label("模型来源"),
                "客户端模型：使用当前供应商与模型，无需重复填写密钥\n智能体原生模型：使用执行主机上的配置与登录状态",
            ).wrap().into();
        acp_options::source_controls(
            &self.state(),
            self.started(),
            help.map(Message::Help),
            Message::Acp,
        )
    }
    fn modal<'a>(&'a self, base: Element<'a, Message>) -> Element<'a, Message> {
        let runtime = &self.runtime;
        let can_change = self.permission_context
            && !runtime.request_lifecycle.is_requesting
            && !runtime.full_access_saving
            && !runtime.acp_options_state.pending
            && runtime.acp_options_state.source_pending.is_none();
        acp_options::with_session_modal(
            &self.state(),
            self.started(),
            base,
            can_change,
            self.source(),
            Message::Acp,
            Message::Send,
        )
    }
    fn overlay(&self) -> Element<'_, Message> {
        let runtime = &self.runtime;
        let popover = model_views::model_popover_content(
            model_views::ModelPopoverInput {
                settings: &self.model_settings,
                i18n: I18n::new(self.desktop_locale),
                cli_local: runtime.acp_agent.is_some()
                    && runtime.acp_model_source == AcpModelSource::CliLocal,
                started: self.started(),
                can_select: model_views::can_select_model(&self.state(), self.started()),
                is_acp: runtime.acp_agent.is_some(),
                input_model: runtime
                    .acp_options_state
                    .model_input_draft
                    .as_deref()
                    .unwrap_or("fixture/model-00")
                    .to_owned(),
            },
            false,
            "fixture/model-00",
            self.source(),
            Message::Model,
        );
        let toggle = model_views::model_toggle_button(
            I18n::new(self.desktop_locale),
            &self.model_settings.providers,
            runtime.acp_agent.is_some() && runtime.acp_model_source == AcpModelSource::CliLocal,
            false,
            "fixture/model-00",
            true,
            Message::Model,
        );
        let overlay = desktop_ui_controls::overlays::AboveOverlay::new(toggle, popover)
            .show(true)
            .block_background_events(true)
            .gap(6.0)
            .on_close(Message::Agent);
        column![
            row![
                Space::new().width(Length::Fill),
                button(text("fixture send")).on_press(Message::Agent)
            ],
            Space::new().height(Length::Fill),
            overlay
        ]
        .padding(10)
        .into()
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_preserves_values_and_disabled_states() {
    if !isolated(&format!("{}::ui_refresh_42_preserves_values_and_disabled_states", module_path!()))
    {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 420);
    for (requesting, pending, theme) in
        [(false, false, Theme::Light), (true, false, Theme::Dark), (false, true, Theme::Dark)]
    {
        let runtime = app.current_session_runtime_mut();
        runtime.acp_options_state.open = true;
        runtime.request_lifecycle.is_requesting = requesting;
        runtime.acp_options_state.pending = pending;
        runtime.acp_options_state.options.access_status = Some(AcpAccessStatus {
            requested: false,
            backend: "fixture".into(),
            version: None,
            support: AcpAccessSupport::Failed,
            explanation: "測試失敗狀態".into(),
        });
        let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
        let mut ui = scene(app.modal(base), &mut renderer, size);
        let layout = bounds(&mut ui, &renderer);
        let viewport = viewport(size);
        let title = label(&layout, "ACP 会话选项");
        let close = label(&layout, "关闭");
        let body = layout.scrolls[0].1;
        inside(title, viewport);
        inside(close, viewport);
        assert!(title.y + title.height < body.y && close.y > body.y + body.height);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("acp-{requesting}-{pending}"),
            Point::ORIGIN,
        );
        scroll(&mut ui, &mut renderer, body.center(), -1500.0);
        let layout = bounds(&mut ui, &renderer);
        assert!(layout.scrolls[0].2.y > 0.0);
        assert_eq!(close, label(&layout, "关闭"));
        let offset = layout.scrolls[0].2;
        let selected = label(&layout, "✓ 當前選項").center() - offset;
        let choice = label(&layout, "待選選項").center() - offset;
        assert!(body.contains(selected) && body.contains(choice));
        hover(&mut ui, &mut renderer, choice);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("acp-scrolled-{requesting}-{pending}"),
            choice,
        );
        assert!(click(&mut ui, &mut renderer, selected).is_empty());
        let selected = click(&mut ui, &mut renderer, choice);
        if requesting || pending {
            assert!(selected.is_empty(), "{selected:?}");
        } else {
            assert!(matches!(
                selected.as_slice(),
                [Message::Acp(acp_options::Action::AcpOptionSelected(selection))]
                    if selection.kind == (AcpOptionKind::Config { id: CONFIG_ID.into() })
                        && selection.value == NEXT_VALUE
            ));
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, close.center()).as_slice(),
            [Message::Acp(acp_options::Action::AcpOptionsClose)]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_acp_permission_modes_reflect_grant_and_readback() {
    if !isolated(&format!(
        "{}::ui_refresh_42_acp_permission_modes_reflect_grant_and_readback",
        module_path!()
    )) {
        return;
    }
    let mut app = fixture();
    app.permission_context = true;
    let mut renderer = renderer().await;
    let size = Size::new(520, 560);
    for (full, busy, support, theme, expected) in [
        (false, false, AcpAccessSupport::Failed, Theme::Light, "同步失败或未确认"),
        (true, false, AcpAccessSupport::Partial, Theme::Dark, "部分支持"),
        (true, true, AcpAccessSupport::Native, Theme::Light, "原生权限已确认"),
    ] {
        let runtime = app.current_session_runtime_mut();
        runtime.acp_options_state.open = true;
        runtime.request_lifecycle.is_requesting = busy;
        runtime.full_access_enabled = full;
        runtime.acp_options_state.options.access_status = Some(AcpAccessStatus {
            requested: full,
            backend: "fixture".into(),
            version: Some("1.0".into()),
            support,
            explanation: "后端权限实测状态".into(),
        });
        runtime.acp_options_state.options.options.truncate(1);
        runtime.acp_options_state.options.options.push(AcpSessionOption {
            kind: AcpOptionKind::Mode,
            name: "供应商原生模式".into(),
            current: "bypass".into(),
            choices: vec![AcpChoice { id: "bypass".into(), name: "绕过审批".into() }],
        });
        let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
        let mut ui = scene(app.modal(base), &mut renderer, size);
        let layout = bounds(&mut ui, &renderer);
        let (selected, other) = if full {
            ("✓ 完全访问权限", "审批模式")
        } else {
            ("✓ 审批模式", "完全访问权限")
        };
        let selected = label(&layout, selected).center();
        let other = label(&layout, other).center();
        label(&layout, &format!("生效能力：{expected}"));
        assert!(!layout.labels.iter().any(|(value, _)| value == "供应商原生模式"));
        capture(&mut ui, &mut renderer, &theme, size, &format!("permission-{full}-{busy}"), other);
        assert!(click(&mut ui, &mut renderer, selected).is_empty());
        let actions = click(&mut ui, &mut renderer, other);
        if busy {
            assert!(actions.is_empty());
        } else {
            assert!(matches!(
                actions.as_slice(),
                [Message::Send(send_message::Message::ToggleFullAccessPermission)]
            ));
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_09_session_model_source_keeps_status_visible() {
    if !isolated(&format!(
        "{}::help_rollout_09_session_model_source_keeps_status_visible",
        module_path!()
    )) {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 600);
    for (locale, theme) in
        [(DesktopLocale::English, Theme::Dark), (DesktopLocale::SimplifiedChinese, Theme::Light)]
    {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let runtime = app.current_session_runtime_mut();
        runtime.acp_options_state.open = true;
        runtime.acp_model_source = AcpModelSource::SessionModel;
        let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
        let mut ui = scene(app.modal(base), &mut renderer, size);
        let layout = bounds(&mut ui, &renderer);
        label(&layout, "✓ 审批模式");
        label(&layout, "请先配置供应商凭据");
        label(&layout, "切换来源将创建新的 ACP 会话；原生上下文不继承");
        label(
            &layout,
            &format!("当前客户端 Provider/model：{}", app.current_session_runtime().model),
        );
        let heading = label(&layout, "模型来源");
        let icon = Point::new(heading.x + heading.width + 15.0, heading.center_y());
        let frame = |ui: &mut Ui<'_, Message>, renderer: &mut Renderer, suffix: &str| {
            capture(
                ui,
                renderer,
                &theme,
                size,
                &format!("help-09-{locale:?}-model-source-{suffix}"),
                Point::ORIGIN,
            )
        };
        let idle = frame(&mut ui, &mut renderer, "idle");
        assert!(click(&mut ui, &mut renderer, icon).is_empty());
        assert!(idle != frame(&mut ui, &mut renderer, "open"), "model source help must open");
        assert!(!layout.labels.iter().any(|(text, _)| text == "使用执行主机上的配置与登录状态"));
        assert!(
            key(&mut ui, &mut renderer, Point::ORIGIN, Key::Named(Named::Escape), None).is_empty()
        );
        assert!(
            idle == frame(&mut ui, &mut renderer, "closed"),
            "Escape must dismiss model source help"
        );
        assert!(
            click(&mut ui, &mut renderer, label(&layout, "智能体原生模型").center()).is_empty()
        );
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&layout, "关闭").center()).as_slice(),
            [Message::Acp(acp_options::Action::AcpOptionsClose)]
        ));
        drop(ui);

        app.current_session_runtime_mut().acp_model_source = AcpModelSource::CliLocal;
        let mut popover = scene(app.overlay(), &mut renderer, size);
        let layout = bounds(&mut popover, &renderer);
        let heading = label(&layout, "模型来源");
        let icon = Point::new(heading.x + heading.width + 15.0, heading.center_y());
        let idle = frame(&mut popover, &mut renderer, "popover-idle");
        assert!(click(&mut popover, &mut renderer, icon).is_empty());
        assert!(
            idle != frame(&mut popover, &mut renderer, "popover-open"),
            "model popover help must open"
        );
        assert!(
            key(&mut popover, &mut renderer, Point::ORIGIN, Key::Named(Named::Escape), None)
                .is_empty()
        );
        assert!(idle == frame(&mut popover, &mut renderer, "popover-closed"));
        assert!(matches!(
            click(&mut popover, &mut renderer, label(&layout, "客户端模型").center()).as_slice(),
            [Message::Acp(acp_options::Action::AcpModelSourceSelected(
                AcpModelSource::SessionModel
            ))]
        ));
        drop(popover);

        let draft = app.current_session_runtime_mut().draft.take();
        let mut locked = scene(app.overlay(), &mut renderer, size);
        let layout = bounds(&mut locked, &renderer);
        label(&layout, "会话开始后不能切换模型");
        let locked_choice = label(&layout, "客户端模型").center();
        let idle = frame(&mut locked, &mut renderer, "locked-idle");
        hover(&mut locked, &mut renderer, locked_choice);
        let hovered = capture(
            &mut locked,
            &mut renderer,
            &theme,
            size,
            &format!("help-09-{locale:?}-model-source-locked-hover"),
            locked_choice,
        );
        assert!(idle != hovered, "locked model source must show a tooltip on hover");
        assert!(click(&mut locked, &mut renderer, locked_choice).is_empty());
        hover(&mut locked, &mut renderer, Point::ORIGIN);
        assert!(idle == frame(&mut locked, &mut renderer, "locked-leave"));
        drop(locked);
        app.current_session_runtime_mut().draft = draft;
    }
}
