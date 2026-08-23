// 验证 MariaDB 记忆后端的标识符校验、模式与连接参数。
use super::*;

#[test]
fn normalize_schema_trims_default_names_and_preserves_custom_names() {
    assert_eq!(normalize_schema("  "), None);
    assert_eq!(normalize_schema(" public "), None);
    assert_eq!(normalize_schema("PUBLIC"), None);
    assert_eq!(normalize_schema("  data_01  "), Some("data_01".to_string()));
    assert_eq!(normalize_schema("PublicData"), Some("PublicData".to_string()));
}

#[test]
fn validate_identifier_accepts_ascii_identifiers_and_rejects_unsafe_values() {
    assert!(validate_identifier("table_123", "table").is_ok());
    assert!(validate_identifier("_scratch9", "table").is_ok());

    for invalid in ["", "9table", "bad name", "bad-name", "bad.name", "name$", "bad:name"] {
        let error = validate_identifier(invalid, "table").unwrap_err().to_string();
        assert!(error.contains("table"));
    }
}

#[test]
fn quote_identifier_and_category_strings_are_stable() {
    assert_eq!(quote_identifier("memories"), "`memories`");
    assert_eq!(quote_identifier("_scratch"), "`_scratch`");
    assert_eq!(MariadbMemory::category_to_str(&MemoryCategory::Core), "core");
    assert_eq!(MariadbMemory::category_to_str(&MemoryCategory::Daily), "daily");
    assert_eq!(MariadbMemory::category_to_str(&MemoryCategory::Conversation), "conversation");
    assert_eq!(
        MariadbMemory::category_to_str(&MemoryCategory::Custom("visual".to_string())),
        "visual"
    );
}

#[test]
fn parse_category_is_case_sensitive_and_preserves_unknown_values() {
    assert_eq!(MariadbMemory::parse_category("core"), MemoryCategory::Core);
    assert_eq!(MariadbMemory::parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(MariadbMemory::parse_category("conversation"), MemoryCategory::Conversation);
    assert_eq!(MariadbMemory::parse_category("Core"), MemoryCategory::Custom("Core".to_string()));
    assert_eq!(MariadbMemory::parse_category(""), MemoryCategory::Custom(String::new()));
}

#[test]
fn new_rejects_invalid_table_before_parsing_connection_url() {
    let error = match MariadbMemory::new("not a mysql url", "public", "bad-table", Some(1), false) {
        Ok(_) => panic!("invalid table should fail"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("storage table"));
    assert!(!error.contains("connection URL"));
}

#[test]
fn new_rejects_invalid_schema_before_parsing_connection_url() {
    let error =
        match MariadbMemory::new("not a mysql url", "bad-schema", "memories", Some(1), false) {
            Ok(_) => panic!("invalid schema should fail"),
            Err(error) => error.to_string(),
        };

    assert!(error.contains("storage schema"));
    assert!(!error.contains("connection URL"));
}

#[test]
fn new_reports_invalid_connection_url_after_identifier_validation() {
    let error = match MariadbMemory::new("not a mysql url", "public", "memories", Some(1), true) {
        Ok(_) => panic!("invalid connection URL should fail"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("invalid MariaDB connection URL"));
}

#[test]
fn new_normalizes_trimmed_public_schema_before_parsing_connection_url() {
    let error = match MariadbMemory::new("not a mysql url", "  PuBlIc\n", "memories", None, false) {
        Ok(_) => panic!("invalid connection URL should fail"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("invalid MariaDB connection URL"));
    assert!(!error.contains("storage schema"));
}

#[test]
fn timeout_cap_constant_documents_the_constructor_bound() {
    assert_eq!(MARIADB_CONNECT_TIMEOUT_CAP_SECS, 300);
}
