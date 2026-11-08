#![cfg(not(target_arch = "wasm32"))]

use super::{
    catalog_api::Event,
    metadata_fill::Event as Fill,
    model_edits_api_tests::{Fixture, load, row},
};
use crate::app::state::app_providers::model_catalog::FillState;
use serde_json::{Value, json};
use vw_gateway_client::vw_api_types::model_catalog::{
    ModelCatalogCacheStatus, ModelMetadataField, ModelMetadataSource,
};

const FIRST: &str = "Vendor/First";
const SECOND: &str = "Vendor/Second";

fn models(fixture: &mut Fixture) {
    let first = row(fixture, FIRST, Some("模型甲"));
    let second = row(fixture, SECOND, Some("模型乙"));
    load(fixture, json!({"revision":"displayed","items":[first,second]}));
}

fn edit(fixture: &mut Fixture, model_id: &str) -> iced::Task<crate::app::Message> {
    fixture.event(Event::EditModel {
        generation: fixture.app.app_providers.generation,
        model_id: model_id.into(),
    })
}

#[test]
fn catalog_desktop_fill_ignores_fill_after_model_change() {
    Fixture::with(|fixture| {
        models(fixture);
        let task = edit(fixture, FIRST);
        assert!(task.units() > 0, "selecting a saved model must request its fill preview");
        let pending = fixture.run(task);
        let request = fixture.request("POST /v1/model-catalog/fill-preview?").unwrap();
        assert_eq!(request["identity"]["modelId"], FIRST);

        let second = edit(fixture, SECOND);
        assert!(second.units() > 0);
        fixture.reply(json!({
            "identity": request["identity"], "revision": "displayed",
            "proposed": {"maxOutputTokens":4096}
        }));
        let result = fixture.finish(pending);
        assert_eq!(result.units(), 0);
        assert_eq!(fixture.form().edit_id.as_ref().unwrap().as_ref(), SECOND);
        assert_eq!(fixture.form().alias, "模型乙");
        assert!(
            matches!(fixture.form().fill, FillState::Loading(_)),
            "late preview must not replace the new model's pending preview"
        );
        assert!(fixture.app.active_toast.is_none());
        assert!(fixture.requests.try_recv().is_err());
    });
}

fn preview(fixture: &mut Fixture, revision: &str, cache: &str, proposed: Value) -> Value {
    let task = edit(fixture, FIRST);
    let pending = fixture.run(task);
    let request = fixture.request("POST /v1/model-catalog/fill-preview?").unwrap();
    fixture.reply(json!({
        "identity": request["identity"], "revision": revision,
        "modelsDevCache": cache, "proposed": proposed,
    }));
    let _ = fixture.finish(pending);
    request
}

fn proposal() -> Value {
    json!({
        "maxOutputTokens":4096, "toolCalls":"supported",
        "provenance":{
            "maxOutputTokens":{"source":"preset"},
            "toolCalls":{"source":"models-dev","fetchedAt":1730000000000_i64}
        }
    })
}

#[test]
fn catalog_desktop_fill_displays_only_proposed_fields_with_their_sources() {
    Fixture::with(|fixture| {
        models(fixture);
        let identity = preview(fixture, "displayed", "stale", proposal());
        assert_eq!(identity["identity"]["deployment"]["targetHostId"], "fixture-mac");
        let FillState::Preview(preview) = &fixture.form().fill else { panic!("missing preview") };
        assert_eq!(preview.models_dev_cache, Some(ModelCatalogCacheStatus::Stale));
        assert_eq!(preview.proposed.context_window, None);
        assert_eq!(preview.proposed.max_output_tokens, Some(4096));
        let source = &preview.proposed.provenance[&ModelMetadataField::ToolCalls];
        assert_eq!(source.source, ModelMetadataSource::ModelsDev);
        assert_eq!(source.fetched_at.unwrap().0, 1730000000000);
        assert_eq!(
            fixture.form().catalog.as_ref().unwrap().items[0].metadata.context_window,
            Some(8192)
        );
        assert!(fixture.requests.try_recv().is_err(), "preview has no write side effect");
    });
}

#[test]
fn catalog_desktop_fill_keeps_draft_and_stays_quiet_offline() {
    Fixture::with(|fixture| {
        models(fixture);
        let task = edit(fixture, FIRST);
        let pending = fixture.run(task);
        fixture.request("POST /v1/model-catalog/fill-preview?").unwrap();
        let _ = fixture.event(Event::Alias("本地草稿".into()));
        fixture.responses.send(None).unwrap();
        let _ = fixture.finish(pending);
        assert!(
            matches!(fixture.form().fill, FillState::Notice(message) if message.contains("离线"))
        );
        assert_eq!(fixture.form().alias, "本地草稿");
        assert_eq!(fixture.form().catalog.as_ref().unwrap().revision, "displayed");
        assert!(fixture.app.active_toast.is_none());
        assert_eq!(fixture.event(Event::Fill(Fill::Commit)).units(), 0);
        assert!(fixture.requests.try_recv().is_err());
    });
}

#[test]
fn catalog_desktop_fill_ignores_preview_after_endpoint_or_host_change() {
    for switch_host in [false, true] {
        Fixture::with(|fixture| {
            models(fixture);
            let task = edit(fixture, FIRST);
            let pending = fixture.run(task);
            let request = fixture.request("POST /v1/model-catalog/fill-preview?").unwrap();
            if switch_host {
                fixture.app.apps_catalog.host = "another-mac".into();
            } else {
                let _ = fixture.event(Event::Endpoint("https://new.example/v1".into()));
            }
            fixture.reply(json!({
                "identity": request["identity"], "revision": "displayed", "proposed":proposal()
            }));
            let _ = fixture.finish(pending);
            if switch_host {
                assert!(fixture.app.app_providers.model_catalog.is_none());
            } else {
                assert!(matches!(fixture.form().fill, FillState::Idle));
                assert!(fixture.form().catalog.is_none());
            }
            assert!(fixture.app.active_toast.is_none());
            assert!(fixture.requests.try_recv().is_err());
        });
    }
}

#[test]
fn catalog_desktop_fill_rejects_revision_changed_during_preview() {
    Fixture::with(|fixture| {
        models(fixture);
        preview(fixture, "newer", "fresh", proposal());
        assert!(
            matches!(fixture.form().fill, FillState::Notice(message) if message.contains("已被修改"))
        );
        assert_eq!(fixture.event(Event::Fill(Fill::Commit)).units(), 0);
        assert!(fixture.requests.try_recv().is_err());
        assert!(fixture.app.active_toast.is_none());
    });
}

#[test]
fn catalog_desktop_fill_noop_never_shows_success() {
    Fixture::with(|fixture| {
        models(fixture);
        preview(fixture, "displayed", "undated", json!({}));
        assert_eq!(fixture.event(Event::Fill(Fill::Commit)).units(), 0);
        assert!(fixture.requests.try_recv().is_err());

        preview(fixture, "displayed", "stale", proposal());
        let task = fixture.event(Event::Fill(Fill::Commit));
        let pending = fixture.run(task);
        fixture.request("POST /v1/model-catalog/fill-commit?").unwrap();
        fixture.reply(json!({"revision":"displayed","changed":false}));
        let _ = fixture.finish(pending);
        assert!(matches!(fixture.form().fill, FillState::Notice("没有新增参数。")));
        assert!(fixture.app.active_toast.is_none());
        assert!(fixture.requests.try_recv().is_err());
    });
}

#[test]
fn catalog_desktop_fill_commits_exact_revision_and_reports_only_real_changes() {
    for silent in [false, true] {
        Fixture::with(|fixture| {
            models(fixture);
            let preview = preview(fixture, "displayed", "undated", proposal());
            let _ = fixture.event(Event::Alias("保留别名草稿".into()));
            let _ = fixture.event(Event::Fill(Fill::Silent(silent)));
            let task = fixture.event(Event::Fill(Fill::Commit));
            let pending = fixture.run(task);
            let commit = fixture.request("POST /v1/model-catalog/fill-commit?").unwrap();
            assert_eq!(
                commit,
                json!({"identity":preview["identity"],"expectedRevision":"displayed"})
            );
            fixture.reply(json!({"revision":"saved","changed":true}));
            fixture.request("GET /v1/model-catalog?");
            let mut first = row(fixture, FIRST, Some("模型甲"));
            first["metadata"]["maxOutputTokens"] = json!(4096);
            let second = row(fixture, SECOND, Some("模型乙"));
            fixture.reply(json!({"revision":"saved","items":[first,second]}));
            let _ = fixture.finish(pending);
            let saved = fixture.form().catalog.as_ref().unwrap();
            assert_eq!(saved.revision, "saved");
            assert_eq!(saved.items[0].metadata.context_window, Some(8192));
            assert_eq!(saved.items[0].metadata.max_output_tokens, Some(4096));
            assert_eq!(saved.items[1].metadata.max_output_tokens, None);
            assert_eq!(fixture.form().alias, "保留别名草稿");
            assert!(matches!(fixture.form().fill, FillState::Notice("已补全空白参数。")));
            assert_eq!(fixture.app.active_toast.is_none(), silent);
            assert!(fixture.requests.try_recv().is_err());
        });
    }
}
