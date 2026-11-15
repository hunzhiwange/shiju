use crate::system_settings_storage::STORAGE_PROVIDER_OPTIONS;
#[test]
fn storage_provider_options_keep_supported_order() {
    assert_eq!(STORAGE_PROVIDER_OPTIONS, ["postgres", "mariadb", "sqlite"]);
}
