use super::apiql::{ApiQlEntityConfig, ApiQlListMode, ApiQlPageDto, ApiQlResponse};
use serde_json::{Value, json};

const PRODUCT_PROPERTY_NAMES: &[&str] = &[
    "id",
    "platformId",
    "companyId",
    "spuNo",
    "skuNo",
    "title",
    "subTitle",
    "specTitle",
    "color",
    "size",
    "spec1",
    "spec2",
    "spec3",
    "images",
    "saleable",
    "status",
    "createAt",
    "updateAt",
    "deleteAt",
    "version",
    "createAccount",
    "updateAccount",
    "attrTitle",
    "attr1",
    "attr2",
    "attr3",
    "attr4",
    "attr5",
    "attr6",
    "attr7",
    "attr8",
    "attr9",
    "attr10",
    "attr11",
    "attr12",
    "attr13",
    "attr14",
    "attr15",
    "fieldTitle",
    "field1",
    "field2",
    "field3",
    "field4",
    "field5",
    "field6",
    "field7",
    "field8",
    "field9",
    "field10",
    "field11",
    "field12",
    "field13",
    "field14",
    "field15",
    "sort",
    "freeFreight",
    "smallUnit",
    "mediumUnit",
    "mediumRatio",
    "largeUnit",
    "largeRatio",
    "salesUnit",
    "baseUnitMinOrder",
    "keyword",
    "isSubsidiary",
    "shelfTime",
    "productModel",
    "locationNo",
    "smallBarcode",
    "mediumBarcode",
    "largeBarcode",
    "smallSalesPrice",
    "smallPurchasePrice",
    "smallMarketPrice",
    "smallMinPrice",
    "smallMaxPrice",
    "mediumSalesPrice",
    "mediumPurchasePrice",
    "mediumMarketPrice",
    "mediumMinPrice",
    "mediumMaxPrice",
    "largeSalesPrice",
    "largePurchasePrice",
    "largeMarketPrice",
    "largeMinPrice",
    "largeMaxPrice",
    "isSpu",
    "isNew",
    "isHot",
    "isSpecial",
    "discount",
    "discountStart",
    "discountEnd",
    "isPreSale",
    "isClientStarDiscount",
    "autoShelfTime",
    "autoOffShelfTime",
    "estimatedArrivalTime",
    "snapshotVersion",
    "brandId",
    "purchaseSupplierId",
    "templateId",
    "collaboratorId",
    "splitOrderNo",
    "fullTitle",
    "skuTitle",
    "productTagId",
    "productTag",
    "productCategoryId",
    "productCategory",
    "productExtend",
];

const PRODUCT_BRAND_PROPERTY_NAMES: &[&str] = &[
    "id",
    "platformId",
    "companyId",
    "brandNo",
    "name",
    "searching",
    "images",
    "createAt",
    "updateAt",
    "deleteAt",
    "createAccount",
    "updateAccount",
    "version",
];

#[test]
fn apiql_response_serializes_php_page_shape() {
    let response = ApiQlResponse {
        data: Vec::<Value>::new(),
        page: Some(ApiQlPageDto {
            per_page: 15,
            current_page: 1,
            total_page: None,
            total_record: None,
            total_macro: 0,
            from: 0,
            to: None,
        }),
        next_cursor: None,
        page_comment: None,
        data_comment: None,
        next_cursor_comment: None,
    };

    let value = serde_json::to_value(response).expect("serialize ApiQL response");

    assert_eq!(
        value,
        json!({
            "data": [],
            "page": {
                "per_page": 15,
                "current_page": 1,
                "total_page": null,
                "total_record": null,
                "total_macro": 0,
                "from": 0,
                "to": null
            }
        })
    );
    assert!(value.get("items").is_none());
}

#[test]
fn apiql_list_modes_round_trip_snake_case() {
    let cases = [
        (ApiQlListMode::Index, "index"),
        (ApiQlListMode::IndexOnly, "index_only"),
        (ApiQlListMode::IndexCount, "index_count"),
        (ApiQlListMode::IndexPage, "index_page"),
    ];

    for (mode, serialized) in cases {
        let value = serde_json::to_value(mode).expect("serialize list mode");
        assert_eq!(value, json!(serialized));

        let round_trip: ApiQlListMode =
            serde_json::from_value(value).expect("deserialize list mode");
        assert_eq!(round_trip, mode);
    }
}

#[test]
fn apiql_entity_config_preserves_full_php_entity_metadata() {
    let raw = json!([
        {
            "entity_class": "product:product",
            "connection_id": "product-main",
            "table": "product",
            "connect": ":data_database",
            "table_name": "Product SKU table",
            "primary_key": "id",
            "auto_increment": "id",
            "soft_delete_column": "delete_at",
            "operation_log_entity": "App\\Product\\Entity\\ProductOperationLog",
            "unique_indexes": {
                "PRIMARY": ["id"],
                "uniq_sku_no": ["platform_id", "company_id", "sku_no"],
                "uniq_spec": ["platform_id", "company_id", "spu_no", "spec_title"]
            },
            "fields": [
                query_field("id", "id", false, true),
                query_field("spuNo", "spu_no", true, true),
                query_field("skuNo", "sku_no", true, true),
                query_field("title", "title", true, true),
                query_field("subTitle", "sub_title", true, true),
                query_field("specTitle", "spec_title", true, true),
                query_field("productModel", "product_model", true, true)
            ],
            "properties": product_properties(),
            "search_fields": ["spuNo", "skuNo", "title", "subTitle", "specTitle", "productModel"],
            "virtual_fields": [
                {
                    "name": "search_key",
                    "column": "spuNo,skuNo,title,subTitle,specTitle,productModel",
                    "virtual_field": true
                }
            ],
            "relations": [
                {
                    "name": "productExtend",
                    "entity_class": "product:product_extend",
                    "kind": "has_one",
                    "local_field": "id",
                    "foreign_field": "productId",
                    "source_key": "id",
                    "target_key": "product_id"
                }
            ],
            "condition_scopes": {
                "productTag": {
                    "name": "productTag",
                    "method": "conditionScopeProductTag",
                    "table": "product_tag",
                    "source_key": "product_id",
                    "target_key": "product_tag_id",
                    "condition_field": "productTagId"
                },
                "productCategory": {
                    "name": "productCategory",
                    "method": "conditionScopeProductCategory",
                    "table": "product_category",
                    "source_key": "product_id",
                    "target_key": "product_category_id",
                    "condition_field": "productCategoryId"
                }
            },
            "lifecycle_events": [
                "beforeCreateEvent",
                "beforeUpdateEvent",
                "afterCreatedEvent",
                "afterUpdatedEvent"
            ],
            "methods": [
                "mediumUnitValidator",
                "mediumRatioValidator",
                "largeRatioValidator",
                "salesUnitValidator",
                "saveProductTag",
                "saveProductCategory",
                "prepareSkuTitle"
            ]
        },
        {
            "entity_class": "product:product_brand",
            "connection_id": "product-main",
            "table": "product_brand",
            "connect": ":data_database",
            "table_name": "Product brand",
            "primary_key": "id",
            "auto_increment": "id",
            "soft_delete_column": "delete_at",
            "unique_indexes": {
                "PRIMARY": ["id"],
                "uniq_id": ["platform_id", "company_id", "brand_no"]
            },
            "fields": [
                query_field("id", "id", false, true),
                query_field("brandNo", "brand_no", true, true),
                query_field("name", "name", true, true),
                query_field("searching", "searching", false, true)
            ],
            "properties": product_brand_properties(),
            "search_fields": ["brandNo", "name"]
        }
    ]);

    let configs: Vec<ApiQlEntityConfig> =
        serde_json::from_value(raw).expect("deserialize full ApiQL entity metadata");
    let product = &configs[0];
    let product_property_names =
        product.properties.iter().map(|field| field.name.as_str()).collect::<Vec<_>>();
    assert_eq!(product_property_names.as_slice(), PRODUCT_PROPERTY_NAMES);
    assert_eq!(product.properties.len(), 111);
    assert_eq!(product.table_name.as_deref(), Some("Product SKU table"));
    assert_eq!(product.primary_key.as_deref(), Some("id"));
    assert_eq!(
        product.unique_indexes["uniq_sku_no"],
        json!(["platform_id", "company_id", "sku_no"])
    );
    assert_eq!(product.lifecycle_events.len(), 4);
    assert!(product.methods.contains(&"prepareSkuTitle".to_string()));

    let spu_no = product.properties.iter().find(|field| field.name == "spuNo").unwrap();
    assert_eq!(spu_no.label.as_deref(), Some("Product code"));
    assert_eq!(spu_no.r#type.as_deref(), Some("string"));
    assert!(spu_no.readonly);
    assert_eq!(spu_no.column_struct["type"], json!("varchar"));
    assert_eq!(spu_no.validator.as_ref().unwrap()["scenes"], json!(["required", "max_length:30"]));
    assert_eq!(spu_no.meta["search_key_column"], json!(true));

    let saleable = product.properties.iter().find(|field| field.name == "saleable").unwrap();
    assert_eq!(saleable.enum_class.as_deref(), Some("YesEnum"));

    let product_tag = product.properties.iter().find(|field| field.name == "productTagId").unwrap();
    assert_eq!(product_tag.virtual_column, Some(json!(true)));
    assert_eq!(product_tag.meta["search_condition_scope"], json!("productTag"));

    let product_extend =
        product.properties.iter().find(|field| field.name == "productExtend").unwrap();
    assert_eq!(product_extend.has_one.as_deref(), Some("ProductExtend"));
    assert_eq!(product_extend.source_key.as_deref(), Some("id"));
    assert_eq!(product_extend.target_key.as_deref(), Some("product_id"));

    let brand = &configs[1];
    let brand_property_names =
        brand.properties.iter().map(|field| field.name.as_str()).collect::<Vec<_>>();
    assert_eq!(brand_property_names.as_slice(), PRODUCT_BRAND_PROPERTY_NAMES);
    assert_eq!(brand.properties.len(), 13);
    assert_eq!(brand.search_fields, vec!["brandNo", "name"]);

    let serialized = serde_json::to_value(&configs).expect("serialize full ApiQL metadata");
    assert_eq!(serialized[0]["properties"].as_array().unwrap().len(), 111);
    assert_eq!(
        serialized[0]["condition_scopes"]["productTag"]["method"],
        json!("conditionScopeProductTag")
    );
    assert_eq!(serialized[1]["properties"].as_array().unwrap().len(), 13);
}

fn query_field(name: &str, column: &str, searchable: bool, sortable: bool) -> Value {
    json!({
        "name": name,
        "column": column,
        "searchable": searchable,
        "sortable": sortable
    })
}

fn product_properties() -> Vec<Value> {
    PRODUCT_PROPERTY_NAMES.iter().map(|name| product_property(name)).collect()
}

fn product_brand_properties() -> Vec<Value> {
    PRODUCT_BRAND_PROPERTY_NAMES.iter().map(|name| product_brand_property(name)).collect()
}

fn product_property(name: &str) -> Value {
    match name {
        "spuNo" => json!({
            "name": name,
            "label": "Product code",
            "type": "string",
            "readonly": true,
            "column_struct": {
                "type": "varchar",
                "default": "",
                "length": 30
            },
            "validator": {
                "scenes": ["required", "max_length:30"],
                "store": null,
                ":update": ["optional"]
            },
            "meta": {
                "search_key_column": true
            }
        }),
        "skuNo" | "title" | "subTitle" | "specTitle" | "productModel" => json!({
            "name": name,
            "type": "string",
            "meta": {
                "search_key_column": true
            }
        }),
        "saleable"
        | "isSpu"
        | "isNew"
        | "isHot"
        | "isSpecial"
        | "isPreSale"
        | "isClientStarDiscount"
        | "freeFreight"
        | "isSubsidiary" => json!({
            "name": name,
            "type": "int",
            "enum_class": "YesEnum"
        }),
        "status" => json!({
            "name": name,
            "type": "int",
            "enum_class": "ProductStatusEnum"
        }),
        "smallUnit" | "mediumUnit" | "largeUnit" | "salesUnit" => json!({
            "name": name,
            "enum_class": "UnitEnum",
            "operation_log": true
        }),
        "productTagId" => json!({
            "name": name,
            "type": "string",
            "virtual_column": true,
            "meta": {
                "search_condition_scope": "productTag"
            }
        }),
        "productCategoryId" => json!({
            "name": name,
            "type": "string",
            "virtual_column": true,
            "meta": {
                "search_condition_scope": "productCategory"
            }
        }),
        "productTag" | "productCategory" => json!({
            "name": name,
            "type": "array",
            "virtual_field": true
        }),
        "productExtend" => json!({
            "name": name,
            "type": "ProductExtend",
            "virtual_field": true,
            "has_one": "ProductExtend",
            "source_key": "id",
            "target_key": "product_id"
        }),
        _ => json!({
            "name": name
        }),
    }
}

fn product_brand_property(name: &str) -> Value {
    match name {
        "brandNo" | "name" => json!({
            "name": name,
            "type": "string",
            "meta": {
                "search_key_column": true
            }
        }),
        "searching" => json!({
            "name": name,
            "type": "int",
            "enum_class": "YesEnum"
        }),
        _ => json!({
            "name": name
        }),
    }
}
