//! Explicit isolated provider list acceptance; never installs production bootstrap.
#![allow(dead_code, unused_imports, clippy::all, clippy::pedantic)]
mod app;
mod apps;
mod desktop_update_helper;
mod fonts;
use app::message::app_providers::list::{ListMessage, message};
use app::{App, Message};
use iced::{Element, Task, Theme};
use vw_gateway_client::vw_api_types::agent_apps::AppId;
fn main() {
    let case = std::env::var("SHIJU_CC04_WINDOW_CASE").expect("window case");
    let (app, native_form) = match case.as_str() {
        "list" | "edit" => (AppId::Claude, false),
        "claude" => (AppId::Claude, true),
        "codex" => (AppId::Codex, true),
        "gemini" => (AppId::Gemini, true),
        "grok" => (AppId::GrokBuild, true),
        "opencode" => (AppId::OpenCode, true),
        "openclaw" => (AppId::OpenClaw, true),
        "hermes" => (AppId::Hermes, true),
        "pi" => (AppId::Pi, true),
        "omp" => (AppId::Omp, true),
        "mcode" => (AppId::Mcode, true),
        "dsh" => (AppId::Dsh, true),
        "kimi" => (AppId::Kimi, true),
        _ => panic!("unknown window case"),
    };
    let edit = case == "edit";
    let theme = match std::env::var("SHIJU_CC04_WINDOW_THEME").as_deref() {
        Ok("dark") => Theme::Dark,
        Ok("light") => Theme::Light,
        _ => panic!("unknown theme"),
    };
    let (port, stop, server) = fixture(native_form, app);
    app::config::install_window_test_bootstrap(port);
    iced::application(
        move || {
            let (mut state, _) = App::new();
            state.apps_catalog.host = "fixture-host".into();
            state.apps_catalog.token = "fake-host-token".into();
            let task =
                app::message::app_providers::list::update(&mut state, ListMessage::Open(app));
            if edit {
                use vw_gateway_client::vw_api_types::app_providers::{ProviderProtocol, create::*};
                state.app_providers.new_draft = Some(AppProviderCreate {
                    name: "fixture".into(),
                    protocol: ProviderProtocol::Anthropic,
                    api_key: ProviderSecretWrite::Keep,
                    base_url: "https://fixture-api.example.test".into(),
                    model: "fixture-model".into(),
                    request_options: None,
                    add_to_live: false,
                });
            }
            (state, task)
        },
        |state: &mut App, event: Message| match event {
            Message::Settings(value) => app::message::settings::update(state, value),
            _ => Task::none(),
        },
        view,
    )
    .title(match case.as_str() {
        "codex" => "Codex 供应商隔离验收",
        "gemini" => "Gemini CLI 供应商隔离验收",
        "grok" => "Grok Build 供应商隔离验收",
        "opencode" => "OpenCode 供应商隔离验收",
        "openclaw" => "OpenClaw 供应商隔离验收",
        "hermes" => "Hermes 供应商隔离验收",
        "pi" => "Pi 供应商隔离验收",
        "omp" => "OMP 供应商隔离验收",
        "mcode" => "Mcode 供应商隔离验收",
        "dsh" => "Dsh 供应商隔离验收",
        "kimi" => "Kimi 供应商隔离验收",
        "claude" => "Claude Code 供应商隔离验收",
        _ => "供应商基础编辑隔离验收",
    })
    .theme(move |_: &App| theme.clone())
    .window_size((1000., 820.))
    .settings(iced::Settings {
        fonts: fonts::load_all(),
        default_font: iced::Font::with_name("PingFang SC"),
        ..Default::default()
    })
    .run()
    .unwrap();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    server.join().unwrap();
}
fn view(state: &App) -> Element<'_, Message> {
    iced::widget::stack![
        app::components::app_providers::list::overlay(
            &state.app_providers,
            iced::widget::container(iced::widget::text("隔离 HTTP SDK 验收"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        ),
        app::components::toast::view(state)
    ]
    .into()
}
fn fixture(
    native_form: bool,
    app: AppId,
) -> (u16, std::sync::Arc<std::sync::atomic::AtomicBool>, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done = stop.clone();
    let server = std::thread::spawn(move || {
        let mut presentation = serde_json::json!({"sortIndex":1,"category":"工作","notes":"中文动态备注：实际字体整形验收。"});
        let mut basics: std::collections::BTreeMap<String, serde_json::Value> = (0..16).map(|i| (format!("provider-{i}"), serde_json::json!({"name":format!("中文供应商 {i}"),"category":"工作","notes":"中文动态备注","websiteUrl":"https://website.example.test"}))).collect();
        let app = match app {
            AppId::Claude => "claude",
            AppId::Codex => "codex",
            AppId::Gemini => "gemini",
            AppId::GrokBuild => "grokbuild",
            AppId::OpenCode => "opencode",
            AppId::OpenClaw => "openclaw",
            AppId::Hermes => "hermes",
            AppId::Pi => "pi",
            AppId::Omp => "omp",
            AppId::Mcode => "mcode",
            AppId::Dsh => "dsh",
            AppId::Kimi => "kimi",
            _ => panic!("unknown fixture app"),
        };
        let prefix = format!("/v1/app-providers/{app}/");
        let mut third_party = serde_json::json!({
            "kind":"third_party","baseUrl":"https://fixture-api.example.test"
        });
        if app == "claude" {
            third_party["apiKeyHeader"] = false.into();
        }
        let mut profiles = std::collections::BTreeMap::from([
            ("provider-0".to_owned(), (third_party, serde_json::json!("现有中文模型"))),
            (
                "provider-1".to_owned(),
                (serde_json::json!({"kind":"official"}), serde_json::Value::Null),
            ),
        ]);
        let mut toml_configs = std::collections::BTreeMap::from([(
            "provider-0".to_owned(),
            serde_json::json!({
                "toml": "model_provider = \"vendor\"\nmodel = \"中文模型\"\n\n[model_providers.vendor]\nbase_url = \"https://fixture-api.example.test/v1\"\nwire_api = \"responses\"\n",
                "hasSavedKey": true,
                "inactive": [["config", "mcp_servers", "中文工具"], ["config", "sandbox_mode"]],
                "models": [{"slug":"fixture-model","displayName":"中文动态映射"}]
            }),
        )]);
        if app == "grokbuild" {
            toml_configs.insert("provider-0".into(), serde_json::json!({
                "toml":"[models]\ndefault = \"vendor\"\n\n[model.vendor]\nmodel = \"中文模型\"\nname = \"中文动态模型\"\nbase_url = \"https://fixture-api.example.test/v1\"\napi_backend = \"responses\"\ncontext_window = 8192\n",
                "hasSavedKey":true,
                "inactive":[["config","legacy","中文旧字段"],["config","ui","theme"]],
            }));
        }
        let incremental = matches!(
            app,
            "opencode" | "openclaw" | "hermes" | "pi" | "omp" | "mcode" | "kimi" | "dsh"
        );
        let mut node_configs = std::collections::BTreeMap::from([(
            "provider-0".to_owned(),
            serde_json::json!({
                "format": "v1",
                "declaration": serde_json::to_string_pretty(&serde_json::json!({
                    "npm": "@ai-sdk/openai-compatible",
                    "name": "中文原生节点",
                    "options": {"baseURL":"https://fixture-api.example.test/v1", "headers":"[保留执行端值]"},
                    "models": {"中文模型":{"name":"中文动态模型", "limit":{"context":8192}}}
                })).unwrap(),
                "hasSavedKey": true,
                "revision": "fixture-1"
            }),
        )]);
        if app == "openclaw" {
            node_configs.insert("provider-0".into(), serde_json::json!({
                "declaration": serde_json::to_string_pretty(&serde_json::json!({
                    "baseUrl":"https://fixture-api.example.test/v1",
                    "api":"openai-completions",
                    "models":[{"id":"fixture-model", "name":"中文动态模型", "contextWindow":8192,
                        "input":["text","image"], "maxTokens":1024, "reasoning":false}]
                })).unwrap(),
                "hasSavedKey": true,
                "userAgent": true,
                "revision": "fixture-1"
            }));
        }
        if app == "hermes" {
            node_configs.insert(
                "provider-0".into(),
                serde_json::json!({
                    "declaration": serde_json::to_string_pretty(&serde_json::json!({
                        "base_url":"https://fixture-api.example.test/v1",
                        "api_mode":"chat_completions",
                        "models":[{"id":"fixture-model","name":"中文动态模型","context_length":8192,
                            "supports_vision":true}],
                        "rate_limit_delay":0.5
                    })).unwrap(),
                    "hasSavedKey":true,
                    "revision":"fixture-1"
                }),
            );
        }
        if app == "pi" {
            node_configs.insert(
                "provider-0".into(),
                serde_json::json!({
                    "declaration": serde_json::to_string_pretty(&serde_json::json!({
                        "baseUrl":"https://fixture-api.example.test/v1",
                        "api":"openai-completions",
                        "models":[{"id":"fixture-model","name":"中文动态模型","contextWindow":8192,
                            "input":["text","image"], "maxTokens":1024, "reasoning":false}]
                    })).unwrap(),
                    "hasSavedKey":true,
                    "revision":"fixture-1"
                }),
            );
        }
        if app == "omp" {
            for id in basics.keys() {
                let models = (0..18)
                    .map(|i| {
                        serde_json::json!({
                            "id": format!("fixture-model-{i}"), "name": format!("中文动态模型 {i}"),
                            "contextWindow": 8192, "maxTokens": 2048
                        })
                    })
                    .collect::<Vec<_>>();
                let declaration = serde_yaml::to_string(&serde_json::json!({
                    "baseUrl": "https://fixture-api.example.test/v1",
                    "api": "openai-completions",
                    "discovery": {"type": "openai-models-list", "injectV1": true},
                    "models": models
                }))
                .unwrap();
                node_configs.insert(
                    id.clone(),
                    serde_json::json!({
                        "declaration": declaration,
                        "hasSavedKey": true,
                        "revision": "fixture-omp"
                    }),
                );
            }
        }
        if matches!(app, "mcode" | "dsh") {
            for id in basics.keys() {
                let models = (0..18).map(|i|(format!("fixture-model-{i}"),serde_json::json!({"name":format!("中文动态模型 {i}"),"limit":{"context":8192,"output":2048}}))).collect::<serde_json::Map<_,_>>();
                let declaration=serde_yaml::to_string(&serde_json::json!({"kind":"custom","api":"openai-completions","options":{"baseURL":"https://fixture-api.example.test/v1"},"models":models})).unwrap();
                node_configs.insert(id.clone(),serde_json::json!({"declaration":declaration,"hasSavedKey":true,"revision":"fixture-mcode"}));
            }
        }
        if app == "kimi" {
            for id in basics.keys() {
                let models = (0..18).map(|i|(format!("fixture-model-{i}"),serde_json::json!({"model":format!("fixture-{i}"),"display_name":format!("中文动态模型 {i}"),"max_context_size":8192}))).collect::<serde_json::Map<_,_>>();
                let declaration=toml::to_string(&serde_json::json!({"provider":{"type":"openai","base_url":"https://fixture-api.example.test/v1"},"models":models})).unwrap();
                node_configs.insert(id.clone(),serde_json::json!({"declaration":declaration,"hasSavedKey":true,"revision":"fixture-kimi"}));
            }
        }
        let mut node_members = std::collections::BTreeSet::from(["provider-0".to_owned()]);
        while !done.load(std::sync::atomic::Ordering::Relaxed) {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                if stream.read_exact(&mut b).is_err() {
                    break;
                }
                head.push(b[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let line = head.lines().next().unwrap_or("");
            let len: usize = head
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut payload = vec![0; len];
            stream.read_exact(&mut payload).unwrap();
            let mut status = "200 OK";
            let path = line.split_whitespace().nth(1).unwrap_or("");
            let id = path.strip_prefix(&prefix).and_then(|v| v.split('/').next()).unwrap_or("");
            let body = if incremental
                && (path.ends_with("/form/read")
                    || path.ends_with("/form")
                    || path.ends_with("/add")
                    || path.ends_with("/remove-live"))
            {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let mut value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(value["targetHostId"], "fixture-host");
                if path.ends_with("/form/read") {
                    assert!(line.starts_with("POST "));
                    serde_json::json!({"fields":basics[id],"config":node_configs.get(id)})
                } else if path.ends_with("/add") || path.ends_with("/remove-live") {
                    assert!(line.starts_with("POST "));
                    if path.ends_with("/remove-live") {
                        node_members.remove(id);
                        serde_json::json!({"catalogSaved":false,"live":"applied"})
                    } else if node_configs.contains_key(id) {
                        node_members.insert(id.into());
                        serde_json::json!({"catalogSaved":false,"live":"applied"})
                    } else {
                        status = "404 Not Found";
                        serde_json::Value::Null
                    }
                } else {
                    assert!(line.starts_with("POST ") || line.starts_with("PATCH "));
                    let declaration = value["declaration"].as_str().unwrap();
                    let node = if app == "kimi" {
                        toml::from_str::<serde_json::Value>(declaration).ok()
                    } else if matches!(app, "omp" | "mcode" | "dsh") {
                        serde_yaml::from_str::<serde_json::Value>(declaration).ok()
                    } else {
                        serde_json::from_str::<serde_json::Value>(declaration).ok()
                    };
                    if !node.and_then(|node| node.get("models").cloned()).is_some_and(|models| {
                        if matches!(app, "openclaw" | "hermes" | "pi" | "omp") {
                            models.as_array().is_some_and(|models| {
                                models.iter().all(|model| {
                                    model["id"].as_str().is_some_and(|id| !id.trim().is_empty())
                                })
                            })
                        } else {
                            models.as_object().is_some_and(|models| {
                                if matches!(app, "mcode" | "kimi" | "dsh") {
                                    !models.is_empty()
                                        && models.iter().all(|(id, model)| {
                                            !id.trim().is_empty() && model.is_object()
                                        })
                                } else {
                                    models.values().all(serde_json::Value::is_object)
                                }
                            })
                        }
                    }) {
                        status = "422 Unprocessable Entity";
                        serde_json::Value::Null
                    } else {
                        let prior = node_configs.get(id);
                        if line.starts_with("PATCH ") {
                            assert_eq!(value["revision"], prior.unwrap()["revision"]);
                        }
                        let has_saved_key = match value["apiKey"]["action"].as_str() {
                            Some("replace") => true,
                            Some("clear") => false,
                            _ => prior.is_some_and(|p| p["hasSavedKey"] == true),
                        };
                        let live = match value["activation"].as_str() {
                            Some("add") => {
                                node_members.insert(id.into());
                                "applied"
                            }
                            Some("if_member") if node_members.contains(id) => "applied",
                            _ => "not_attempted",
                        };
                        let mut config = serde_json::json!({
                            "declaration": declaration,
                            "hasSavedKey": has_saved_key,
                            "revision": "fixture-saved"
                        });
                        match app {
                            "openclaw" => config["userAgent"] = value["userAgent"].clone(),
                            "opencode" => config["format"] = value["format"].clone(),
                            _ => {}
                        }
                        node_configs.insert(id.into(), config);
                        basics.insert(id.into(), value["fields"].take());
                        serde_json::json!({"catalogSaved":true,"live":live})
                    }
                }
            } else if native_form && path.ends_with("/form/read") && line.starts_with("POST ") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let target: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(target["targetHostId"], "fixture-host");
                if matches!(app, "codex" | "grokbuild") {
                    serde_json::json!({"fields":basics[id],"config":toml_configs.get(id)})
                } else {
                    let (config, model) = profiles.get(id).cloned().unwrap_or((
                        serde_json::json!({"kind":"unsupported"}),
                        serde_json::Value::Null,
                    ));
                    serde_json::json!({"fields":basics[id],"config":config,"model":model})
                }
            } else if native_form
                && path.ends_with("/form")
                && (line.starts_with("POST ") || line.starts_with("PATCH "))
            {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let mut value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                assert_eq!(value["targetHostId"], "fixture-host");
                if matches!(app, "codex" | "grokbuild") {
                    let toml = value["toml"].as_str().unwrap();
                    if !toml.contains("https://fixture-api.example.test/v1")
                        || toml.contains("model = [")
                        || (app == "grokbuild" && toml.contains("context_window = 0"))
                    {
                        status = "422 Unprocessable Entity";
                        serde_json::Value::Null
                    } else {
                        let prior = toml_configs.get(id);
                        let has_saved_key = match value["apiKey"]["action"].as_str() {
                            Some("replace") => true,
                            Some("clear") => false,
                            _ => prior.is_some_and(|p| p["hasSavedKey"] == true),
                        };
                        let mut config = serde_json::json!({
                            "toml":toml, "hasSavedKey":has_saved_key,
                            "inactive":prior.map_or_else(|| serde_json::json!([]), |p| p["inactive"].clone()),
                        });
                        if app == "codex" {
                            config["models"] = value["models"].take();
                        }
                        toml_configs.insert(id.into(), config);
                        basics.insert(id.into(), value["fields"].take());
                        serde_json::json!({"catalogSaved":true,"live":if value["apply"] == true {"applied"}else{"not_attempted"}})
                    }
                } else {
                    let mut config = value["config"].take();
                    if config["kind"] == "third_party"
                        && !config["baseUrl"]
                            .as_str()
                            .is_some_and(|url| url.starts_with("https://"))
                    {
                        status = "422 Unprocessable Entity";
                        serde_json::Value::Null
                    } else {
                        config.as_object_mut().unwrap().remove("apiKey");
                        profiles.insert(id.into(), (config, value["model"].take()));
                        basics.insert(id.into(), value["fields"].take());
                        if line.starts_with("POST ") {
                            status = "201 Created";
                        }
                        serde_json::json!({"catalogSaved":true,"live":"not_attempted"})
                    }
                }
            } else if path.ends_with("/basic") && line.starts_with("GET ") {
                basics.get(id).cloned().unwrap_or_else(|| {
                    status = "404 Not Found";
                    serde_json::json!({"message":"missing"})
                })
            } else if path.ends_with("/basic") && line.starts_with("PUT ") {
                let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                if value["fields"]["name"] == "fail" {
                    status = "403 Forbidden";
                    serde_json::json!({"message":"denied"})
                } else {
                    if value["apiKey"]["action"] == "keep" {
                        assert!(value["apiKey"].get("value").is_none());
                    }
                    basics.insert(id.into(), value["fields"].clone());
                    value["fields"].clone()
                }
            } else if line.starts_with("DELETE ") {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                if id == "provider-0" {
                    status = "409 Conflict";
                    serde_json::json!({"message":"referenced"})
                } else {
                    basics.remove(id);
                    serde_json::json!({"deleted":true})
                }
            } else if line.starts_with("GET ")
                && path.starts_with(&format!("{prefix}catalog/state?"))
            {
                assert!(head.to_lowercase().contains("x-agent-apps-token: fake-host-token"));
                let selection = if incremental {
                    serde_json::json!({"status":"not_applicable"})
                } else {
                    serde_json::json!({"status":"selected","id":"provider-0"})
                };
                serde_json::json!({"appId":app,"rows":basics.iter().map(|(id, fields)|serde_json::json!({
                    "provider":{"id":id,"appId":app,"name":fields["name"],"configured":true},
                    "nativeMember":if (incremental && node_members.contains(id)) || (!incremental && id=="provider-0") {"present"}else{"absent"}
                })).collect::<Vec<_>>(),"direct":selection,"inUse":selection,"nativeRead":null})
            } else if line.starts_with("GET ") && path == format!("{prefix}presentation/entries") {
                serde_json::json!([{"id":"provider-0","presentation":presentation}])
            } else if line.starts_with("PUT ")
                && path.starts_with(&prefix)
                && line.contains("/presentation ")
            {
                let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                if value["notes"] == "fail" {
                    status = "403 Forbidden";
                    serde_json::json!({"error":"denied"})
                } else {
                    presentation = value;
                    presentation.clone()
                }
            } else {
                status = "404 Not Found";
                serde_json::json!({"error":"unknown fixture path"})
            };
            let body = body.to_string();
            write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (port, stop, server)
}
