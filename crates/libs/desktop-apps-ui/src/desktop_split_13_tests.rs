#[test]
fn cookie_config_example_documents_required_cookie_fields() {
    let example = crate::bookmark_modals::WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE;

    assert!(example.starts_with('['));
    assert!(example.ends_with(']'));
    for field in ["\"name\"", "\"domain\"", "\"days\"", "\"url_filter\""] {
        assert!(example.contains(field));
    }
}

use crate::aggregation::clients_view::overlay;
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
    let form = crate::aggregation::clients::Session::default();
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
