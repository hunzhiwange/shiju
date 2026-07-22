//! Schema 清理器测试模块
//!
//! 本模块提供针对 `SchemaCleanr` 工具的全面测试覆盖，验证 JSON Schema 在不同
//! AI 提供商策略下的清理和转换行为。
//!
//! # 主要功能
//!
//! - 测试不支持的 JSON Schema 关键字移除
//! - 测试 `$ref` 引用解析与展开
//! - 测试联合类型的扁平化处理
//! - 测试 `null` 类型的处理策略
//! - 测试 `const` 到 `enum` 的转换
//! - 测试元数据字段的保留
//! - 测试循环引用防护
//! - 测试 Schema 有效性校验
//! - 测试不同提供商策略的差异
//!
//! # 提供商策略
//!
//! - **Gemini 策略**：最严格，移除大部分验证关键字
//! - **OpenAI 策略**：最宽松，保留验证关键字
//!
//! # 相关模块
//!
//! - [`super::super`] - 父模块，包含 `SchemaCleanr` 实现

use super::super::*;
use serde_json::json;

/// 测试移除不支持的 JSON Schema 关键字
///
/// 验证 `SchemaCleanr::clean_for_gemini` 能够正确移除 Gemini 不支持的
/// 验证关键字（如 `minLength`、`maxLength`、`pattern`），同时保留
/// 基本类型信息和描述字段。
///
/// # 测试场景
///
/// - 输入包含 `type`、`minLength`、`maxLength`、`pattern`、`description`
/// - 期望输出保留 `type` 和 `description`
/// - 期望输出移除所有验证关键字
#[test]
fn test_remove_unsupported_keywords() {
    // 构造包含验证关键字的字符串类型 Schema
    let schema = json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 100,
        "pattern": "^[a-z]+$",
        "description": "A lowercase string"
    });

    // 使用 Gemini 策略清理 Schema
    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证基本类型和描述被保留
    assert_eq!(cleaned["type"], "string");
    assert_eq!(cleaned["description"], "A lowercase string");
    // 验证验证关键字被移除
    assert!(cleaned.get("minLength").is_none());
    assert!(cleaned.get("maxLength").is_none());
    assert!(cleaned.get("pattern").is_none());
}

/// 测试 `$ref` 引用解析
///
/// 验证 `SchemaCleanr` 能够正确解析并展开 `$ref` 引用，将引用替换为
/// 实际定义的内容，同时移除不再需要的 `$defs` 定义块。
///
/// # 测试场景
///
/// - 输入包含指向 `$defs` 的 `$ref` 引用
/// - 期望引用被展开为实际定义
/// - 期望 `$defs` 块被移除
/// - 期望嵌套的验证关键字被清理
#[test]
fn test_resolve_ref() {
    // 构造包含 $ref 引用的 Schema
    let schema = json!({
        "type": "object",
        "properties": {
            "age": {
                "$ref": "#/$defs/Age"
            }
        },
        "$defs": {
            "Age": {
                "type": "integer",
                "minimum": 0
            }
        }
    });

    // 清理 Schema 并解析引用
    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证引用被正确展开
    assert_eq!(cleaned["properties"]["age"]["type"], "integer");
    // Gemini 策略会移除 minimum 验证关键字
    assert!(cleaned["properties"]["age"].get("minimum").is_none());
    // $defs 定义块应被移除
    assert!(cleaned.get("$defs").is_none());
}

/// 测试字面量联合类型的扁平化
///
/// 验证 `SchemaCleanr` 能够将 `anyOf` 形式的字面量联合类型简化为
/// 更简洁的 `enum` 数组形式，提高 Schema 可读性。
///
/// # 测试场景
///
/// - 输入为 `anyOf` 包含多个 `const` 字面量
/// - 期望输出转换为 `type` + `enum` 形式
/// - 期望枚举值保持原有顺序和内容
#[test]
fn test_flatten_literal_union() {
    // 构造 anyOf 字面量联合类型
    let schema = json!({
        "anyOf": [
            { "const": "admin", "type": "string" },
            { "const": "user", "type": "string" },
            { "const": "guest", "type": "string" }
        ]
    });

    // 扁平化为 enum 形式
    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证类型被提取为顶层
    assert_eq!(cleaned["type"], "string");
    // 验证转换为 enum 数组
    assert!(cleaned["enum"].is_array());
    let enum_values = cleaned["enum"].as_array().unwrap();
    assert_eq!(enum_values.len(), 3);
    // 验证所有枚举值都被保留
    assert!(enum_values.contains(&json!("admin")));
    assert!(enum_values.contains(&json!("user")));
    assert!(enum_values.contains(&json!("guest")));
}

/// 测试从联合类型中移除 `null`
///
/// 验证 `SchemaCleanr` 能够将包含 `null` 类型的联合类型简化，
/// 移除 `null` 选项，简化为单一非空类型。
///
/// # 测试场景
///
/// - 输入为 `oneOf` 包含 `string` 和 `null`
/// - 期望输出简化为 `{ "type": "string" }`
/// - 期望 `oneOf` 结构被移除
#[test]
fn test_strip_null_from_union() {
    // 构造包含 null 的联合类型
    let schema = json!({
        "oneOf": [
            { "type": "string" },
            { "type": "null" }
        ]
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 简化为单一类型
    assert_eq!(cleaned["type"], "string");
    // oneOf 结构应被移除
    assert!(cleaned.get("oneOf").is_none());
}

/// 测试 `const` 到 `enum` 的转换
///
/// 验证 `SchemaCleanr` 能够将单一 `const` 值转换为 `enum` 数组形式，
/// 保持描述等元数据字段不变。
///
/// # 测试场景
///
/// - 输入包含 `const` 和 `description`
/// - 期望 `const` 转换为单元素 `enum` 数组
/// - 期望 `description` 被保留
/// - 期望原始 `const` 字段被移除
#[test]
fn test_const_to_enum() {
    // 构造 const 类型的 Schema
    let schema = json!({
        "const": "fixed_value",
        "description": "A constant"
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证转换为 enum 数组
    assert_eq!(cleaned["enum"], json!(["fixed_value"]));
    // 描述应被保留
    assert_eq!(cleaned["description"], "A constant");
    // const 字段应被移除
    assert!(cleaned.get("const").is_none());
}

/// 测试元数据字段的保留
///
/// 验证 `SchemaCleanr` 在解析引用时能够正确保留并合并元数据字段，
/// 如 `description`、`title`、`default` 等。
///
/// # 测试场景
///
/// - 输入 `$ref` 引用带有顶层元数据
/// - 期望引用展开后元数据被保留
/// - 期望引用目标的类型信息被合并
#[test]
fn test_preserve_metadata() {
    // 构造带元数据的引用 Schema
    let schema = json!({
        "$ref": "#/$defs/Name",
        "description": "User's name",
        "title": "Name Field",
        "default": "Anonymous",
        "$defs": {
            "Name": {
                "type": "string"
            }
        }
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证类型从引用目标获取
    assert_eq!(cleaned["type"], "string");
    // 验证所有元数据字段被保留
    assert_eq!(cleaned["description"], "User's name");
    assert_eq!(cleaned["title"], "Name Field");
    assert_eq!(cleaned["default"], "Anonymous");
}

/// 测试循环引用防护
///
/// 验证 `SchemaCleanr` 能够安全处理自引用或循环引用的 Schema，
/// 不会导致无限递归或栈溢出。
///
/// # 测试场景
///
/// - 输入包含自引用的 Schema（Node 引用自身）
/// - 期望清理过程不会 panic
/// - 期望引用被正确解析到有限深度
#[test]
fn test_circular_ref_prevention() {
    // 构造包含循环引用的 Schema
    let schema = json!({
        "type": "object",
        "properties": {
            "parent": {
                "$ref": "#/$defs/Node"
            }
        },
        "$defs": {
            "Node": {
                "type": "object",
                "properties": {
                    "child": {
                        "$ref": "#/$defs/Node"
                    }
                }
            }
        }
    });

    // 循环引用不应导致 panic
    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证引用被解析
    assert_eq!(cleaned["properties"]["parent"]["type"], "object");
    // 循环引用应被正确处理（通过深度限制或已访问追踪）
}

/// 测试 Schema 有效性校验
///
/// 验证 `SchemaCleanr::validate` 方法能够正确识别有效和无效的
/// JSON Schema 结构。
///
/// # 测试场景
///
/// - 有效 Schema：包含必需的 `type` 字段
/// - 无效 Schema：缺少必需的 `type` 字段
#[test]
fn test_validate_schema() {
    // 构造有效的 Schema（包含 type 字段）
    let valid = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    });

    assert!(SchemaCleanr::validate(&valid).is_ok());

    // 构造无效的 Schema（缺少 type 字段）
    let invalid = json!({
        "properties": {
            "name": { "type": "string" }
        }
    });

    assert!(SchemaCleanr::validate(&invalid).is_err());
}

/// 测试不同提供商策略的差异
///
/// 验证 `SchemaCleanr` 针对不同 AI 提供商（Gemini、OpenAI）的
/// 清理策略差异。Gemini 策略更严格，OpenAI 策略更宽松。
///
/// # 测试场景
///
/// - 同一 Schema 使用两种策略清理
/// - Gemini 策略应移除验证关键字（如 `minLength`）
/// - OpenAI 策略应保留验证关键字
/// - 两种策略都应保留基本类型和描述
#[test]
fn test_strategy_differences() {
    let schema = json!({
        "type": "string",
        "minLength": 1,
        "description": "A string field"
    });

    // Gemini 策略：最严格，移除 minLength
    let gemini = SchemaCleanr::clean_for_gemini(schema.clone());
    assert!(gemini.get("minLength").is_none());
    assert_eq!(gemini["type"], "string");
    assert_eq!(gemini["description"], "A string field");

    // OpenAI 策略：最宽松，保留 minLength
    let openai = SchemaCleanr::clean_for_openai(schema.clone());
    // OpenAI 允许保留验证关键字
    assert_eq!(openai["minLength"], 1);
    assert_eq!(openai["type"], "string");
}

/// 测试嵌套属性的处理
///
/// 验证 `SchemaCleanr` 能够递归处理嵌套对象的属性，确保
/// 清理策略应用到所有层级的字段。
///
/// # 测试场景
///
/// - 输入包含多层嵌套的对象 Schema
/// - 期望内层属性的验证关键字被清理
/// - 期望 `additionalProperties` 被移除（Gemini 不支持）
#[test]
fn test_nested_properties() {
    // 构造嵌套对象的 Schema
    let schema = json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "minLength": 1
                    }
                },
                "additionalProperties": false
            }
        }
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证嵌套属性中的验证关键字被移除
    assert!(cleaned["properties"]["user"]["properties"]["name"].get("minLength").is_none());
    // 验证 additionalProperties 被移除
    assert!(cleaned["properties"]["user"].get("additionalProperties").is_none());
}

/// 测试类型数组中 `null` 的移除
///
/// 验证 `SchemaCleanr` 能够处理 `type` 为数组形式的可空类型，
/// 将 `["string", "null"]` 简化为 `"string"`。
///
/// # 测试场景
///
/// - 输入 `type` 为 `["string", "null"]`
/// - 期望输出简化为 `"string"`
#[test]
fn test_type_array_null_removal() {
    // 构造类型数组形式的可空类型
    let schema = json!({
        "type": ["string", "null"]
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 简化为单一非空类型
    assert_eq!(cleaned["type"], "string");
}

/// 测试仅包含 `null` 的类型数组
///
/// 验证当类型数组仅包含 `null` 时，能够正确保留 `null` 类型，
/// 而非产生空或无效的输出。
///
/// # 测试场景
///
/// - 输入 `type` 为 `["null"]`
/// - 期望输出保留 `"null"` 类型
#[test]
fn test_type_array_only_null_preserved() {
    // 构造仅包含 null 的类型
    let schema = json!({
        "type": ["null"]
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // null 类型应被保留
    assert_eq!(cleaned["type"], "null");
}

/// 测试带 JSON Pointer 转义的 `$ref` 引用
///
/// 验证 `SchemaCleanr` 能够正确处理 `$ref` 中包含特殊字符（如 `/`）
/// 的 JSON Pointer 转义序列（`~1` 表示 `/`）。
///
/// # 测试场景
///
/// - 输入 `$ref` 使用 `~1` 转义
/// - `$defs` 中的键名包含 `/` 字符
/// - 期望引用被正确解析
#[test]
fn test_ref_with_json_pointer_escape() {
    // 构造包含转义字符的引用
    let schema = json!({
        "$ref": "#/$defs/Foo~1Bar",
        "$defs": {
            "Foo/Bar": {
                "type": "string"
            }
        }
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证转义序列被正确解析
    assert_eq!(cleaned["type"], "string");
}

/// 测试存在不可简化联合类型时跳过类型设置
///
/// 验证当 Schema 包含无法简化的复杂联合类型时，
/// `SchemaCleanr` 不会错误地设置顶层 `type` 字段。
///
/// # 测试场景
///
/// - 输入包含 `oneOf` 联合类型，且包含顶层 `type`
/// - 联合类型成员为复杂对象，无法扁平化
/// - 期望输出保留 `oneOf` 结构
/// - 期望移除可能冲突的顶层 `type`
#[test]
fn test_skip_type_when_non_simplifiable_union_exists() {
    // 构造包含复杂联合类型的 Schema
    let schema = json!({
        "type": "object",
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "a": { "type": "string" }
                }
            },
            {
                "type": "object",
                "properties": {
                    "b": { "type": "number" }
                }
            }
        ]
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 当存在不可简化的联合类型时，不应设置顶层 type
    assert!(cleaned.get("type").is_none());
    // oneOf 结构应被保留
    assert!(cleaned.get("oneOf").is_some());
}

/// 测试清理嵌套的未知 Schema 关键字
///
/// 验证 `SchemaCleanr` 能够处理包含未知关键字（如 `not`）的 Schema，
/// 并递归清理其中的引用和验证关键字。
///
/// # 测试场景
///
/// - 输入包含 `not` 关键字，其值为 `$ref` 引用
/// - 期望引用被展开
/// - 期望嵌套的验证关键字被清理
#[test]
fn test_clean_nested_unknown_schema_keyword() {
    // 构造包含 not 关键字的 Schema
    let schema = json!({
        "not": {
            "$ref": "#/$defs/Age"
        },
        "$defs": {
            "Age": {
                "type": "integer",
                "minimum": 0
            }
        }
    });

    let cleaned = SchemaCleanr::clean_for_gemini(schema);

    // 验证 not 中的引用被展开
    assert_eq!(cleaned["not"]["type"], "integer");
    // 验证验证关键字被移除
    assert!(cleaned["not"].get("minimum").is_none());
}

#[test]
fn unresolved_ref_preserves_metadata_without_leaking_ref() {
    let cleaned = SchemaCleanr::clean_for_gemini(json!({
        "$ref": "https://example.com/schemas/Remote",
        "description": "remote value",
        "title": "Remote"
    }));

    assert_eq!(
        cleaned,
        json!({
            "description": "remote value",
            "title": "Remote"
        })
    );
}

#[test]
fn definitions_ref_decodes_tilde_and_cleans_target() {
    let cleaned = SchemaCleanr::clean_for_gemini(json!({
        "$ref": "#/definitions/Tilde~0Name",
        "definitions": {
            "Tilde~Name": {
                "type": "string",
                "format": "hostname"
            }
        }
    }));

    assert_eq!(cleaned, json!({"type": "string"}));
}

#[test]
fn provider_strategies_expose_expected_keyword_boundaries() {
    assert!(CleaningStrategy::Anthropic.unsupported_keywords().contains(&"$ref"));
    assert!(!CleaningStrategy::Anthropic.unsupported_keywords().contains(&"minLength"));
    assert!(
        CleaningStrategy::Conservative.unsupported_keywords().contains(&"additionalProperties")
    );
    assert!(CleaningStrategy::OpenAI.unsupported_keywords().is_empty());
}

#[test]
fn clean_handles_non_object_values_and_multi_type_arrays() {
    assert_eq!(SchemaCleanr::clean_for_gemini(json!("string")), json!("string"));
    assert_eq!(
        SchemaCleanr::clean_for_gemini(json!([{"type": "string", "pattern": "x"}, 7])),
        json!([{"type": "string"}, 7])
    );
    assert_eq!(
        SchemaCleanr::clean_for_gemini(json!({"type": ["string", "integer", "null"]})),
        json!({"type": ["string", "integer"]})
    );
}

#[test]
fn validate_rejects_non_object_schema() {
    let error =
        SchemaCleanr::validate(&json!(["string"])).expect_err("array schema should be rejected");

    assert_eq!(error.to_string(), "Schema 必须是一个对象");
}
