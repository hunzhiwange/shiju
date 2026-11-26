//! 任务表单和推理等级的真实布局、键盘事件与截图路径。
use super::ui_driver::*;
use crate::input_panel::{reasoning_selector, task_mode};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_settings_providers::state::{ModelSettingsState, ModelSummary, ProviderModelsSummary};
use iced::advanced::widget::Id;
use iced::keyboard::{Key, key::Named};
use iced::widget::container;
use iced::{Point, Size, Theme};
use vw_api_acp_model::{
    acp_model_source::{AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason},
    acp_session::{AcpChoice, AcpOptionKind, AcpSessionOption},
};
use vw_api_chat_identity::ReasoningEffortDto;
#[derive(Debug, Clone)]
enum Message {
    Task(task_mode::Message),
    Reasoning(crate::input_panel::reasoning_message::Message),
}
const CONFIG_ID: &str = "fixture/backend-option";
const NEXT_VALUE: &str = "fixture/backend-value-2";
struct Fixture {
    runtime: desktop_session::session::agent::SessionRuntimeState<()>,
    model_settings: ModelSettingsState,
    desktop_locale: DesktopLocale,
    app_theme: Theme,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            runtime: desktop_session::session::agent::SessionRuntimeState::new(),
            model_settings: Default::default(),
            desktop_locale: DesktopLocale::SimplifiedChinese,
            app_theme: Theme::Dark,
        }
    }
}
impl Fixture {
    fn current_session_runtime_mut(
        &mut self,
    ) -> &mut desktop_session::session::agent::SessionRuntimeState<()> {
        &mut self.runtime
    }
    fn current_session_runtime_ref(
        &self,
    ) -> Option<&desktop_session::session::agent::SessionRuntimeState<()>> {
        Some(&self.runtime)
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_layout_and_interaction() {
    if !isolated(&format!("{}::ui_refresh_42_layout_and_interaction", module_path!())) {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (width, viewport_width, locale, theme) in [
        (420, 420, DesktopLocale::TraditionalChinese, Theme::Dark),
        (640, 664, DesktopLocale::SimplifiedChinese, Theme::Light),
        (960, 984, DesktopLocale::English, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let size = Size::new(viewport_width, 560);
        let form = task_mode::task_mode_form(
            task_mode::Input {
                i18n,
                theme: &app.app_theme,
                providers: &app.model_settings.providers,
                agents: &[],
                expanded: false,
            },
            true,
            "10",
            "fixture/model-00",
            Some("codex".into()),
            None,
        )
        .map(Message::Task);
        let mut ui = scene(container(form).padding(12).into(), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("task-{width}"), Point::ORIGIN);

        let reasoning = reasoning_selector::reasoning_effort_control(
            i18n,
            Some("fixture-a"),
            "fixture/model-00",
            &[None, Some(ReasoningEffortDto::High)],
            None,
        );
        let mut ui = scene(
            container(reasoning.map(Message::Reasoning)).width(260).into(),
            &mut renderer,
            size,
        );
        capture(&mut ui, &mut renderer, &theme, size, &format!("reasoning-{width}"), Point::ORIGIN);
        assert!(matches!(
            click(&mut ui, &mut renderer, Point::new(234.0, 52.0)).as_slice(),
            [Message::Reasoning(crate::input_panel::reasoning_message::Message::Selected(Some(
                ReasoningEffortDto::High
            )))]
        ));
    }
}
