use crate::app::message::settings::aggregation_pi::Form;
use iced::widget::text_editor::Content;
use vw_gateway_client::vw_api_types::agent_apps::aggregation::{
    AggregationQuery, PiDirectoryChange, PiDirectoryState,
};
#[test]
fn aggregation_t65_aggregation_pi_preserves_contract() {
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let form = Form {
        snapshot: Some(PiDirectoryState {
            revision: "opaque-revision".into(),
            members: vec!["managed".into()],
            default_provider_id: Some("native".into()),
            default_model_id: Some("原生模型".into()),
        }),
        id: "managed".into(),
        name: "中文名称".into(),
        declaration: Content::with_text(r#"{"models":[{"id":"model"}]}"#),
        ..Default::default()
    };
    let (_, write) = form.write(&query, false).unwrap();
    assert!(matches!(write.change, PiDirectoryChange::Replace { .. }));
    assert_eq!(write.expected_revision, "opaque-revision");
    assert_eq!(form.snapshot.as_ref().unwrap().default_provider_id.as_deref(), Some("native"));
}

#[test]
fn aggregation_t65_requires_revision_and_confirmed_member_for_delete() {
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let mut form = Form::default();
    assert!(form.write(&query, false).is_err());
    form.snapshot = Some(PiDirectoryState {
        revision: "r1".into(),
        members: vec!["managed".into()],
        default_provider_id: Some("native".into()),
        default_model_id: None,
    });
    assert!(form.write(&query, true).is_err());
    form.removing = Some("native".into());
    assert!(form.write(&query, true).is_err());
    form.removing = Some("managed".into());
    let (id, write) = form.write(&query, true).unwrap();
    assert_eq!(id, "managed");
    assert!(matches!(write.change, PiDirectoryChange::Remove {}));
    assert_eq!(
        serde_json::to_value(write).unwrap()["change"],
        serde_json::json!({"kind":"remove"})
    );
}

#[tokio::test]
async fn aggregation_t65_sdk_wire_and_conflict_preserve_draft_without_replay() {
    use crate::app::message::settings::aggregation_pi::execute;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
    };
    use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        for (path, response, status) in [
            (
                "POST /v1/app-providers/pi/config/state ",
                r#"{"revision":"r1","members":["managed"],"defaultProviderId":"native","defaultModelId":"原生模型"}"#,
                "200 OK",
            ),
            (
                "PUT /v1/app-providers/pi/managed/directory-entry ",
                r#"{"error":{"code":"revision_conflict","message":"sensitive fixture"}}"#,
                "409 Conflict",
            ),
            (
                "POST /v1/app-providers/pi/config/state ",
                r#"{"revision":"r2","members":["managed"],"defaultProviderId":"native","defaultModelId":"原生模型"}"#,
                "200 OK",
            ),
            (
                "PUT /v1/app-providers/pi/managed/directory-entry ",
                r#"{"revision":"r3","members":[],"defaultProviderId":"native","defaultModelId":"原生模型"}"#,
                "200 OK",
            ),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with(path));
            let mut len = 0;
            let mut auth = false;
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                let header = line.to_ascii_lowercase();
                if let Some(value) = header.strip_prefix("content-length:") {
                    len = value.trim().parse().unwrap();
                }
                auth |= header.trim() == "authorization: bearer fixture";
            }
            assert!(auth);
            let mut bytes = vec![0; len];
            reader.read_exact(&mut bytes).unwrap();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["projectId"], "project");
            assert_eq!(body["targetHostId"], "host");
            if path.starts_with("PUT") {
                if status == "409 Conflict" {
                    assert_eq!(body["expectedRevision"], "r1");
                    assert_eq!(body["change"]["kind"], "replace");
                } else {
                    assert_eq!(body["expectedRevision"], "r2");
                    assert_eq!(body["change"], serde_json::json!({"kind":"remove"}));
                }
                assert!(body.get("defaultModelId").is_none());
                assert!(body.get("account").is_none());
            }
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
        }
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture".into()) }),
    )
    .unwrap();
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let state = execute(client.clone(), query.clone(), None).await.unwrap();
    let mut form = Form {
        snapshot: Some(state),
        id: "managed".into(),
        name: "中文草稿".into(),
        declaration: Content::with_text(r#"{"models":[{"id":"模型"}]}"#),
        pending: Some(7),
        ..Default::default()
    };
    let write = form.write(&query, false).unwrap();
    let result = execute(client.clone(), query.clone(), Some(write)).await;
    assert!(matches!(
        &result,
        Err(vw_gateway_client::AggregationApiError::Http { status: 409, .. })
    ));
    assert!(!form.finish(6, result.clone()));
    assert_eq!(form.pending, Some(7));
    assert!(form.finish(7, result));
    assert_eq!(form.id, "managed");
    assert_eq!(form.name, "中文草稿");
    assert!(form.declaration.text().contains("模型"));
    assert!(form.snapshot.is_none());
    assert!(form.write(&query, false).is_err());
    assert!(!form.error.unwrap().contains("sensitive"));
    assert!(form.error.unwrap().contains("目录已变化"));
    form.snapshot = Some(execute(client.clone(), query.clone(), None).await.unwrap());
    form.removing = Some("managed".into());
    let remove = form.write(&query, true).unwrap();
    let after = execute(client, query, Some(remove)).await.unwrap();
    assert!(after.members.is_empty());
    assert_eq!(after.default_provider_id.as_deref(), Some("native"));
    assert_eq!(after.default_model_id.as_deref(), Some("原生模型"));
    server.join().unwrap();
}

#[tokio::test]
async fn aggregation_t65_renders_dynamic_chinese_in_light_and_dark() {
    use iced::{
        Font, Length, Size, Theme,
        advanced::renderer::Headless,
        mouse,
        widget::{container, text},
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    let form = Form {
        snapshot: Some(PiDirectoryState {
            revision: "r1".into(),
            members: vec!["managed".into()],
            default_provider_id: Some("原生供应商".into()),
            default_model_id: Some("动态中文模型".into()),
        }),
        id: "managed".into(),
        name: "中文名称".into(),
        declaration: Content::with_text(r#"{"models":[{"id":"模型","name":"中文模型"}]}"#),
        ..Default::default()
    };
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    #[derive(Default)]
    struct Labels {
        text: Vec<String>,
        inputs: usize,
    }
    impl iced::advanced::widget::Operation for Labels {
        fn traverse(
            &mut self,
            children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
        ) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: iced::Rectangle, text: &str) {
            self.text.push(text.into());
        }
        fn text_input(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            _: iced::Rectangle,
            _: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs += 1;
        }
    }
    let mut images = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        let mut ui = UserInterface::build(
            crate::app::components::aggregation_pi::overlay(
                &form,
                container(text("背景")).width(Length::Fill).height(Length::Fill).into(),
            ),
            Size::new(900., 800.),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        for label in ["Pi 模型目录", "原生供应商", "动态中文模型", "保存单条"] {
            assert!(labels.text.iter().any(|value| value == label), "missing modal label: {label}");
        }
        assert_eq!(labels.inputs, 2, "only provider ID and display name are editable text inputs");
        ui.draw(
            &mut renderer,
            &theme,
            &iced::advanced::renderer::Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let pixels = renderer.screenshot(Size::new(900, 800), 1., theme.palette().background);
        assert_eq!(pixels.len(), 900 * 800 * 4);
        let central_changes = (100..700)
            .flat_map(|y| (100..800).map(move |x| (y * 900 + x) * 4))
            .filter(|offset| pixels[*offset..*offset + 4] != pixels[..4])
            .count();
        assert!(
            central_changes > 5_000,
            "modal must paint inside the viewport, not just the background label"
        );
        image::save_buffer(
            format!("/tmp/shiju-t65-{}.png", if theme == Theme::Light { "light" } else { "dark" }),
            &pixels,
            900,
            800,
            image::ColorType::Rgba8,
        )
        .unwrap();
        images.push(pixels);
    }
    assert!(images[0] != images[1]);
}

#[test]
fn aggregation_t65_permission_target_and_decode_errors_require_refresh() {
    use vw_gateway_client::AggregationApiError;
    for error in [
        AggregationApiError::Http { status: 403, body: serde_json::json!({"secret":"fixture"}) },
        AggregationApiError::Http { status: 422, body: serde_json::Value::Null },
        AggregationApiError::Http { status: 503, body: serde_json::Value::Null },
        AggregationApiError::Decode("fixture".into()),
    ] {
        let mut form = Form {
            id: "managed".into(),
            name: "保留草稿".into(),
            pending: Some(9),
            ..Default::default()
        };
        assert!(form.finish(9, Err(error)));
        assert!(form.pending.is_none());
        assert!(form.snapshot.is_none());
        assert_eq!(form.name, "保留草稿");
        assert!(!form.error.unwrap().contains("fixture"));
    }
}

#[test]
fn aggregation_t65_insert_keeps_model_declaration_and_rejects_invalid_drafts() {
    let query = AggregationQuery { target_host_id: "host".into(), project_id: "project".into() };
    let mut form = Form {
        snapshot: Some(PiDirectoryState {
            revision: "r1".into(),
            members: vec![],
            default_provider_id: None,
            default_model_id: None,
        }),
        id: "new-provider".into(),
        name: "新模型".into(),
        declaration: Content::with_text(r#"{"models":[{"id":"model","name":"中文模型"}]}"#),
        ..Default::default()
    };
    let (_, write) = form.write(&query, false).unwrap();
    let PiDirectoryChange::Insert { profile } = write.change else {
        panic!("new entry must insert")
    };
    assert_eq!(
        profile.declaration["models"][0],
        serde_json::json!({"id":"model","name":"中文模型"})
    );
    form.declaration = Content::with_text("{broken");
    assert!(form.write(&query, false).is_err());
    assert_eq!(form.declaration.text(), "{broken");
    form.declaration = Content::with_text("[]");
    assert!(form.write(&query, false).is_err());
    form.id = "../outside".into();
    assert!(form.write(&query, false).is_err());
}
