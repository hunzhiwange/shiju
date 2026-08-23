// 覆盖 ApiQL 模式注册、租户持久化与配置校验。

use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};
use vw_api_types::id::TenantScope;

use crate::gateway::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;

use super::schema::{
    APIQL_ENTITY_SETTINGS_NAMESPACE, ApiQlEntityConfigStore, ApiQlEntityRegistry,
    ApiQlSortDirection, load_entity_configs, load_entity_registry, local_entity_config_store,
    save_entity_configs,
};

fn field(name: &str, column: &str, searchable: bool, sortable: bool) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(column.to_string()),
        searchable,
        sortable,
        virtual_field: false,
        ..ApiQlFieldConfig::default()
    }
}

fn virtual_field(name: &str, dependencies: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig {
        name: name.to_string(),
        column: Some(dependencies.to_string()),
        searchable: false,
        sortable: false,
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    }
}

fn product_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "product:product".to_string(),
        connection_id: "product-main".to_string(),
        table: "product".to_string(),
        fields: vec![
            field("id", "id", false, true),
            field("name", "name", true, true),
            field("sku", "sku", true, false),
            field("created_at", "created_at", false, true),
        ],
        search_fields: vec!["name".to_string(), "sku".to_string()],
        virtual_fields: vec![
            virtual_field("search_key", "name,sku"),
            virtual_field("summary", "search_key,created_at"),
        ],
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn invoice_config() -> ApiQlEntityConfig {
    ApiQlEntityConfig {
        entity_class: "billing:invoice".to_string(),
        connection_id: "billing-main".to_string(),
        table: "invoice".to_string(),
        fields: vec![field("id", "id", false, true)],
        search_fields: Vec::new(),
        virtual_fields: Vec::new(),
        relations: Vec::new(),
        ..ApiQlEntityConfig::default()
    }
}

fn temp_store() -> (tempfile::TempDir, GlobalDatabase, ApiQlEntityConfigStore) {
    let dir = tempfile::tempdir().expect("temporary ApiQL config directory");
    let database =
        GlobalDatabase::new(dir.path().join("database.sqlite3")).expect("global database");
    let store = ApiQlEntityConfigStore::new(database.clone(), TenantScope::default())
        .expect("ApiQL entity config store");
    (dir, database, store)
}

#[test]
fn apiql_schema_resolves_registered_entity() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity configs should be valid");

    let resolved = registry.resolve("product:product").expect("registered entity");
    assert_eq!(resolved.entity_key(), "product:product");
    assert_eq!(resolved.config().connection_id, "product-main");
    assert_eq!(resolved.config().table, "product");
    assert_eq!(resolved.search_field_names(), vec!["name", "sku"]);
    assert_eq!(
        resolved.virtual_field_dependencies("summary").expect("virtual deps"),
        vec!["name", "sku", "created_at"],
    );

    let default_order = resolved
        .parse_default_order("created_at desc, id asc")
        .expect("default order should parse");
    assert_eq!(default_order[0].field, "created_at");
    assert_eq!(default_order[0].direction, ApiQlSortDirection::Desc);
    assert_eq!(default_order[1].field, "id");
    assert_eq!(default_order[1].direction, ApiQlSortDirection::Asc);
    assert_eq!(APIQL_ENTITY_SETTINGS_NAMESPACE, "apiql.entities");
}

#[test]
fn apiql_schema_rejects_unknown_entity() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity configs should be valid");

    let error = registry.resolve("billing:invoice").expect_err("unknown entity should fail");
    assert_eq!(error.to_string(), "ApiQL entity not registered: billing:invoice");

    let error = registry.resolve("product").expect_err("malformed entity key should fail");
    assert_eq!(error.to_string(), "Invalid ApiQL entity key");
}

#[test]
fn apiql_entity_configs_round_trip_through_gateway_settings() {
    let (dir, _, store) = temp_store();
    let legacy_path = dir.path().join("storage/apiql/entities/product/product.json");
    std::fs::create_dir_all(legacy_path.parent().expect("legacy entity parent"))
        .expect("legacy entity directory");
    let mut legacy_product = product_config();
    legacy_product.table = "legacy_product".to_string();
    std::fs::write(&legacy_path, serde_json::to_vec(&legacy_product).expect("legacy entity JSON"))
        .expect("legacy entity input");
    assert!(
        store.load_configs().expect("empty database entity list").is_empty(),
        "legacy entity JSON must not populate an empty database"
    );

    let database_product = product_config();
    let invoice = invoice_config();
    store
        .save_configs(&[database_product.clone(), invoice.clone()])
        .expect("save database entity configs");
    store
        .save_configs(std::slice::from_ref(&database_product))
        .expect("upsert one entity without replacing the list");

    let loaded = store.load_configs().expect("load database entity configs");
    let registry = ApiQlEntityRegistry::from_configs(loaded.clone())
        .expect("database entity configs should validate");

    assert_eq!(loaded, vec![invoice, database_product]);
    assert_eq!(
        registry.resolve("product:product").expect("database product").config().table,
        "product"
    );
    assert!(registry.resolve("billing:invoice").is_ok());
    assert!(legacy_path.exists());
}

#[test]
fn apiql_entity_configs_are_isolated_by_tenant_with_the_same_entity_key() {
    let dir = tempfile::tempdir().expect("temporary ApiQL config directory");
    let database =
        GlobalDatabase::new(dir.path().join("database.sqlite3")).expect("global database");
    let first_tenant = TenantScope::new("apiql-platform-a", "apiql-company-a").expect("first");
    let second_tenant = TenantScope::new("apiql-platform-b", "apiql-company-b").expect("second");
    let first = ApiQlEntityConfigStore::new(database.clone(), first_tenant).expect("first store");
    let second = ApiQlEntityConfigStore::new(database, second_tenant).expect("second store");
    let first_config = product_config();
    let mut second_config = product_config();
    second_config.table = "tenant_b_product".to_string();

    first.save_configs(std::slice::from_ref(&first_config)).expect("save first tenant");
    second.save_configs(std::slice::from_ref(&second_config)).expect("save second tenant");

    assert_eq!(first.load_configs().expect("load first tenant"), vec![first_config]);
    assert_eq!(second.load_configs().expect("load second tenant"), vec![second_config]);
}

#[test]
fn apiql_schema_rejects_invalid_entity_and_field_configs() {
    let mut config = product_config();
    config.connection_id = " ".to_string();
    assert!(
        ApiQlEntityRegistry::from_configs(vec![config])
            .unwrap_err()
            .to_string()
            .contains("empty connection_id")
    );

    let mut config = product_config();
    config.table = " ".to_string();
    assert!(
        ApiQlEntityRegistry::from_configs(vec![config])
            .unwrap_err()
            .to_string()
            .contains("empty table")
    );

    let mut config = product_config();
    config.fields[0].name = "invalid.field".to_string();
    assert!(
        ApiQlEntityRegistry::from_configs(vec![config])
            .unwrap_err()
            .to_string()
            .contains("Invalid ApiQL field name")
    );

    let mut config = product_config();
    config.search_fields.push("missing".to_string());
    assert_eq!(
        ApiQlEntityRegistry::from_configs(vec![config]).unwrap_err().to_string(),
        "Unknown ApiQL search field: missing"
    );

    let mut config = product_config();
    config.entity_class = "Product:product".to_string();
    assert!(
        ApiQlEntityRegistry::from_configs(vec![config])
            .unwrap_err()
            .to_string()
            .contains("Invalid ApiQL entity key")
    );

    let mut config = product_config();
    config.virtual_fields[0].name = "search.key".to_string();
    assert!(
        ApiQlEntityRegistry::from_configs(vec![config])
            .unwrap_err()
            .to_string()
            .contains("Invalid ApiQL field name")
    );

    assert_eq!(
        ApiQlEntityRegistry::from_configs(vec![product_config(), product_config()])
            .unwrap_err()
            .to_string(),
        "Duplicate ApiQL entity config"
    );
}

#[test]
fn apiql_schema_reports_order_and_virtual_dependency_errors() {
    let registry = ApiQlEntityRegistry::from_configs(vec![product_config()])
        .expect("entity config should be valid");
    let entity = registry.resolve("product:product").expect("registered entity");

    assert!(
        entity
            .parse_default_order("created_at sideways")
            .unwrap_err()
            .to_string()
            .contains("Invalid ApiQL default order direction")
    );
    assert_eq!(
        entity.parse_default_order("sku desc").unwrap_err().to_string(),
        "ApiQL field is not sortable: sku"
    );
    assert_eq!(
        entity.parse_default_order("missing asc").unwrap_err().to_string(),
        "Unknown ApiQL order field: missing"
    );
    assert_eq!(
        entity.parse_default_order("id asc extra").unwrap_err().to_string(),
        "Invalid ApiQL default order"
    );
    assert_eq!(
        entity.parse_default_order(" , id").expect("blank order segments are skipped")[0].field,
        "id"
    );

    let mut config = product_config();
    config.virtual_fields =
        vec![virtual_field("first", "second"), virtual_field("second", "first")];
    let registry = ApiQlEntityRegistry::from_configs(vec![config]).expect("cycle validates lazily");
    let error = registry
        .resolve("product:product")
        .unwrap()
        .virtual_field_dependencies("first")
        .unwrap_err();
    assert!(error.to_string().contains("Circular ApiQL virtual field dependency"));

    let mut config = product_config();
    config.virtual_fields = vec![virtual_field("broken", "missing")];
    let registry =
        ApiQlEntityRegistry::from_configs(vec![config]).expect("dependency validates lazily");
    assert_eq!(
        registry
            .resolve("product:product")
            .unwrap()
            .virtual_field_dependencies("broken")
            .unwrap_err()
            .to_string(),
        "Unknown ApiQL virtual field dependency: missing"
    );
}

#[test]
fn apiql_schema_uses_searchable_fields_and_deletes_saved_config() {
    let mut config = product_config();
    config.search_fields.clear();
    let registry = ApiQlEntityRegistry::from_configs(vec![config.clone()])
        .expect("entity config should be valid");
    assert_eq!(
        registry.resolve("product:product").unwrap().search_field_names(),
        vec!["name", "sku"]
    );

    let (_dir, _, store) = temp_store();
    store.save_configs(&[config]).expect("save entity config");
    store.delete_config("product:product").expect("delete entity config");
    assert!(store.load_configs().expect("load entity configs").is_empty());
}

#[test]
fn apiql_schema_load_configs_reports_database_payload_errors() {
    let (_dir, database, store) = temp_store();
    let settings = GatewaySettingsStore::new(database).expect("gateway settings store");
    let tenant = TenantScope::default();

    settings
        .upsert(&tenant, APIQL_ENTITY_SETTINGS_NAMESPACE, "product:product", "{not-json")
        .expect("insert invalid ApiQL JSON");
    let error = store.load_configs().expect_err("invalid JSON should fail");
    assert!(error.to_string().contains("Invalid ApiQL entity config in shared database"));

    settings
        .upsert(
            &tenant,
            APIQL_ENTITY_SETTINGS_NAMESPACE,
            "product:product",
            &serde_json::to_string(&invoice_config()).expect("invoice config JSON"),
        )
        .expect("insert mismatched entity JSON");
    let error = store.load_configs().expect_err("key mismatch should fail");
    assert_eq!(
        error.to_string(),
        "ApiQL entity config key does not match entity_class: expected product:product, got billing:invoice"
    );

    settings
        .upsert(
            &tenant,
            APIQL_ENTITY_SETTINGS_NAMESPACE,
            "Product:product",
            &serde_json::to_string(&product_config()).expect("product config JSON"),
        )
        .expect("insert invalid config key");
    let error = store.load_configs().expect_err("invalid key should fail first");
    assert!(error.to_string().contains("Invalid ApiQL entity key: Product:product"));
}

#[test]
fn apiql_schema_save_and_delete_validate_entity_keys() {
    let (_dir, _, store) = temp_store();
    let mut config = product_config();
    config.entity_class = "Product:product".to_string();

    let error = store.save_configs(&[config]).expect_err("invalid save key should fail");
    assert!(error.to_string().contains("Invalid ApiQL entity key: Product:product"));

    let error = store.delete_config("product").expect_err("invalid delete key should fail");
    assert_eq!(error.to_string(), "Invalid ApiQL entity key");
}

#[tokio::test]
async fn apiql_schema_async_helpers_round_trip_through_global_store() {
    let tenant =
        TenantScope::new("apiql-schema-platform", "apiql-schema-company").expect("tenant scope");
    let config = product_config();

    save_entity_configs(tenant.clone(), std::slice::from_ref(&config))
        .await
        .expect("save entity configs through async helper");
    let loaded = load_entity_configs(tenant.clone()).await.expect("load entity configs");
    let registry = load_entity_registry(tenant.clone()).await.expect("load entity registry");

    assert_eq!(loaded, vec![config]);
    assert_eq!(
        registry.resolve("product:product").expect("registered entity").config().table,
        "product"
    );

    local_entity_config_store(tenant)
        .expect("local entity store")
        .delete_config("product:product")
        .expect("cleanup global entity config");
}

#[test]
fn apiql_schema_matches_order_and_virtual_field_aliases() {
    let mut config = product_config();
    config.fields.push(field("HTTPStatus", "legacy-status", false, true));
    config.fields.push(field("createdAt", "legacy_created", false, true));
    config.virtual_fields.push(ApiQlFieldConfig {
        name: "HTTPStatusLabel".to_string(),
        column: Some("HTTPStatus".to_string()),
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    });
    config.virtual_fields.push(ApiQlFieldConfig {
        name: "emptyDeps".to_string(),
        column: None,
        virtual_field: true,
        ..ApiQlFieldConfig::default()
    });
    let registry = ApiQlEntityRegistry::from_configs(vec![config]).expect("alias config");
    let entity = registry.resolve("product:product").expect("registered entity");

    let order = entity
        .parse_default_order("http_status desc, legacy-status asc, legacy.created desc")
        .expect("order aliases should resolve");
    assert_eq!(order[0].field, "HTTPStatus");
    assert_eq!(order[0].direction, ApiQlSortDirection::Desc);
    assert_eq!(order[1].field, "HTTPStatus");
    assert_eq!(order[2].field, "createdAt");

    assert_eq!(
        entity.virtual_field_dependencies("http.status.label").expect("dot virtual alias"),
        vec!["HTTPStatus"],
    );
    assert!(entity.virtual_field_dependencies("emptyDeps").expect("empty deps").is_empty());
    assert!(
        entity
            .virtual_field_dependencies("missingVirtual")
            .unwrap_err()
            .to_string()
            .contains("Unknown ApiQL virtual field")
    );
}
