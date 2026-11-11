use super::*;
#[tokio::test]
async fn aggregation_t62_aggregation_clients_preserves_contract() {
    use iced::{Size, advanced::renderer::Headless};
    use iced_runtime::user_interface::{Cache, UserInterface};
    #[derive(Default)]
    struct Probe(Vec<String>);
    impl iced::advanced::widget::operation::Operation for Probe {
        fn traverse(
            &mut self,
            f: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
        ) {
            f(self);
        }
        fn text(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            _: iced::Rectangle,
            value: &str,
        ) {
            self.0.push(value.into());
        }
    }
    let selection = crate::app::message::settings::aggregation_list::Selection {
        scope: crate::app::state::app_providers::list::Scope {
            context: crate::app::state::AgentAppsContext {
                gateway: "http://fixture".into(),
                tenant: Default::default(),
                project: None,
            },
            host: "host".into(),
            app: vw_gateway_client::vw_api_types::agent_apps::AppId::Codex,
        },
        project: "project".into(),
        catalog_generation: 1,
    };
    let mut form = Form::new(selection, 1);
    form.loading = false;
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        overlay(&form, iced::widget::Space::new().into()),
        Size::new(960.0, 800.0),
        Cache::new(),
        &mut renderer,
    );
    let mut probe = Probe::default();
    ui.operate(&renderer, &mut probe);
    assert!(probe.0.iter().any(|s| s == "Codex 客户端状态"), "missing scoped stale-client entry");
}
