// 覆盖 ApiQL 实体配置、字段映射与注册表解析。

use serde_json::{Map, Value, json};
use vw_api_types::apiql::{ApiQlEntityConfig, ApiQlFieldConfig};

use super::{prepare_sku_title, resolve_business_virtual_field};
use crate::gateway::api::handlers::apiql::schema::ApiQlEntityRegistry;

fn registry(entity_class: &str) -> ApiQlEntityRegistry {
    ApiQlEntityRegistry::from_configs(vec![ApiQlEntityConfig {
        entity_class: entity_class.to_string(),
        connection_id: "main".to_string(),
        table: "products".to_string(),
        ..ApiQlEntityConfig::default()
    }])
    .expect("valid entity config")
}

fn virtual_field(name: &str) -> ApiQlFieldConfig {
    ApiQlFieldConfig { name: name.to_string(), virtual_field: true, ..Default::default() }
}

#[test]
fn prepare_sku_title_handles_empty_invalid_and_non_collection_inputs() {
    assert_eq!(prepare_sku_title("  "), Some(String::new()));
    assert_eq!(prepare_sku_title("not-json"), None);
    assert_eq!(prepare_sku_title("42"), Some(String::new()));
}

#[test]
fn prepare_sku_title_collects_non_empty_values_after_the_first_colon() {
    assert_eq!(
        prepare_sku_title(r#"["Color: Red", "Size: M: Tall", "Missing", "Empty: ", 7]"#),
        Some("Red M".to_string())
    );
}

#[test]
fn product_virtual_fields_accept_snake_and_camel_aliases() {
    let registry = registry("product:product");
    let entity = registry.resolve("product:product").expect("product entity");
    let row = Map::from_iter([
        ("title".to_string(), json!("Shirt")),
        ("specTitle".to_string(), json!(r#"["Color: Blue", "Size: L"]"#)),
    ]);

    let sku = resolve_business_virtual_field(entity, &virtual_field("sku_title"), &row)
        .expect("sku title should resolve");
    let full = resolve_business_virtual_field(entity, &virtual_field("fullTitle"), &row)
        .expect("full title should resolve");

    assert_eq!(sku, Some(json!("Blue L")));
    assert_eq!(full, Some(json!("Shirt Blue L")));
}

#[test]
fn business_virtual_field_returns_none_for_unknown_entity_or_field() {
    let other_registry = registry("catalog:item");
    let other = other_registry.resolve("catalog:item").expect("other entity");
    let product_registry = registry("product:product");
    let product = product_registry.resolve("product:product").expect("product entity");
    let row = Map::<String, Value>::new();

    assert_eq!(
        resolve_business_virtual_field(other, &virtual_field("skuTitle"), &row)
            .expect("unknown entity is not an error"),
        None
    );
    assert_eq!(
        resolve_business_virtual_field(product, &virtual_field("unknown"), &row)
            .expect("unknown field is not an error"),
        None
    );
}
