use super::import::*;
#[test]
fn cc04_import_dto_has_separate_startup_and_rejects_paths() {
    let b = serde_json::json!({"targetHostId":"host", "trigger":"startup"});
    assert_eq!(
        serde_json::from_value::<ProviderImport>(b.clone()).unwrap().trigger,
        ImportTrigger::Startup
    );
    let mut b = b;
    b["path"] = serde_json::json!("/tmp/source");
    assert!(serde_json::from_value::<ProviderImport>(b).is_err());
    assert_eq!(
        serde_json::to_value(ProviderImportResult { imported: 1 }).unwrap(),
        serde_json::json!({"imported":1})
    );
}
