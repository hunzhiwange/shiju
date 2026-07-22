//! # 委托协调状态工具测试模块
//!
//! 本模块包含 `DelegateCoordinationStatusTool` 的集成测试用例，
//! 用于验证委托协调状态工具的各种功能，包括：
//!
//! - 上下文和收件箱状态报告
//! - 死信队列限制和分页
//! - 上下文条目的限制、排序和分页
//! - 基于关联ID的过滤和分页
//! - 消息分页与关联ID过滤
//!
//! ## 测试覆盖范围
//!
//! 1. **基础状态报告**：验证工具能正确报告上下文条目和收件箱状态
//! 2. **死信处理**：验证死信队列的限制和分页功能
//! 3. **上下文管理**：验证上下文条目的最近访问顺序排序和分页
//! 4. **过滤功能**：验证基于关联ID的过滤和分页功能
//! 5. **消息分页**：验证收件箱消息的分页和过滤功能

use super::super::*;
use crate::app::agent::coordination::{
    CoordinationEnvelope, CoordinationPayload, InMemoryMessageBus, SequencedEnvelope,
};
use serde_json::json;

/// 创建用于测试的内存消息总线实例
///
/// 该辅助函数初始化一个 `InMemoryMessageBus` 并注册两个测试代理：
/// - `delegate-lead`：委托主导代理，用于发送委托任务和状态更新
/// - `researcher`：研究代理，用于接收委托任务
///
/// # 返回值
///
/// 返回已注册测试代理的 `InMemoryMessageBus` 实例
///
/// # 示例
///
/// ```ignore
/// let bus = test_bus();
/// // bus 已预注册 "delegate-lead" 和 "researcher" 代理
/// ```
fn test_bus() -> InMemoryMessageBus {
    let bus = InMemoryMessageBus::new();
    bus.register_agent("delegate-lead").expect("register lead should succeed");
    bus.register_agent("researcher").expect("register researcher should succeed");
    bus
}

/// 测试状态工具报告上下文和收件箱信息
///
/// # 测试场景
///
/// 1. 创建委托任务请求消息并发送到研究代理
/// 2. 发送上下文补丁更新委托状态
/// 3. 执行状态工具并验证返回的JSON包含正确的：
///    - 收件箱信息（1个收件箱，1条待处理消息）
///    - 上下文计数（1个上下文条目）
///    - 分页元数据（偏移量、限制、截断标志等）
///    - 统计信息（发布尝试、投递、驱逐等）
///
/// # 验证点
///
/// - `include_messages: true` 能正确返回收件箱消息
/// - `agent` 和 `correlation_id` 过滤器能正确筛选数据
/// - 所有分页字段都正确设置
/// - 统计数据准确反映操作历史
#[tokio::test]
async fn status_tool_reports_context_and_inboxes() {
    let bus = test_bus();

    // 创建委托任务请求消息：从 delegate-lead 发送到 researcher
    let mut request = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "researcher",
        "delegate:corr-1",
        "delegate.request",
        CoordinationPayload::DelegateTask {
            task_id: "corr-1".to_string(),
            summary: "Investigate".to_string(),
            metadata: json!({"priority":"high"}),
        },
    );
    request.correlation_id = Some("corr-1".to_string());
    bus.publish(request).expect("request should publish");

    // 创建上下文补丁：更新委托状态为 "queued"
    let mut patch = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-1",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-1/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch.correlation_id = Some("corr-1".to_string());
    bus.publish(patch).expect("state patch should publish");

    // 执行状态工具，请求包含消息、指定代理和关联ID
    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "include_messages": true,
            "agent": "researcher",
            "correlation_id": "corr-1"
        }))
        .await
        .expect("tool execution should succeed");

    // 验证执行成功
    assert!(result.success);

    // 解析输出JSON并验证各字段
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    // 验证收件箱信息
    assert_eq!(parsed["inboxes"].as_array().map(Vec::len), Some(1));
    assert_eq!(parsed["inboxes"][0]["pending"], json!(1));
    assert_eq!(parsed["inboxes"][0]["pending_filtered"], json!(1));
    assert_eq!(parsed["inboxes"][0]["message_total"], json!(1));
    assert_eq!(parsed["inboxes"][0]["message_offset"], json!(0));
    assert_eq!(parsed["inboxes"][0]["messages_returned"], json!(1));
    assert_eq!(parsed["inboxes"][0]["messages_truncated"], json!(false));
    assert_eq!(parsed["inboxes"][0]["message_next_offset"], serde_json::Value::Null);

    // 验证上下文计数
    assert_eq!(parsed["context_count"], json!(1));
    assert_eq!(parsed["delegate_context_count"], json!(1));
    assert_eq!(parsed["delegate_context_count_filtered"], json!(1));
    assert_eq!(parsed["contexts_total"], json!(1));
    assert_eq!(parsed["contexts_offset"], json!(0));
    assert_eq!(parsed["contexts_returned"], json!(1));
    assert_eq!(parsed["contexts_truncated"], json!(false));
    assert_eq!(parsed["context_next_offset"], serde_json::Value::Null);
    assert_eq!(parsed["contexts"].as_array().map(Vec::len), Some(1));

    // 验证死信队列信息
    assert_eq!(parsed["dead_letters_total"], json!(0));
    assert_eq!(parsed["dead_letters_returned"], json!(0));
    assert_eq!(parsed["dead_letters_truncated"], json!(false));
    assert_eq!(parsed["dead_letter_next_offset"], serde_json::Value::Null);

    // 验证限制配置
    assert_eq!(parsed["limits"]["max_inbox_messages_per_agent"], json!(256));
    assert_eq!(parsed["limits"]["max_dead_letters"], json!(256));
    assert_eq!(parsed["limits"]["max_context_entries"], json!(512));
    assert_eq!(parsed["limits"]["max_seen_message_ids"], json!(4096));

    // 验证统计信息
    assert_eq!(parsed["stats"]["publish_attempts_total"], json!(2));
    assert_eq!(parsed["stats"]["deliveries_total"], json!(2));
    assert_eq!(parsed["stats"]["dead_letters_total"], json!(0));
    assert_eq!(parsed["stats"]["dead_letter_evictions_total"], json!(0));
    assert_eq!(parsed["stats"]["context_evictions_total"], json!(0));
    assert_eq!(parsed["stats"]["seen_message_id_evictions_total"], json!(0));
}

/// 测试状态工具对死信队列的限制功能
///
/// # 测试场景
///
/// 1. 发布3条无效消息（缺少关联ID），它们会被放入死信队列
/// 2. 使用 `dead_letter_limit: 2` 执行状态工具
/// 3. 验证只返回前2条死信，但总数显示为3
///
/// # 验证点
///
/// - 死信总数 (`dead_letters_total`) 为3
/// - 返回的死信数 (`dead_letters_returned`) 为2（受limit限制）
/// - 截断标志 (`dead_letters_truncated`) 为 true
/// - 下一页偏移量 (`dead_letter_next_offset`) 为2
/// - 统计信息正确反映死信数量
#[tokio::test]
async fn status_tool_applies_dead_letter_limit() {
    let bus = test_bus();

    // 发布3条无效消息：缺少 correlation id 会导致消息进入死信队列
    for index in 0..3 {
        let mut invalid = CoordinationEnvelope::new_direct(
            "delegate-lead",
            "researcher",
            format!("delegate:corr-{index}"),
            "delegate.result",
            CoordinationPayload::TaskResult {
                task_id: format!("corr-{index}"),
                success: false,
                output: "failure".to_string(),
            },
        );
        invalid.id = format!("invalid-{index}");
        // 缺少关联ID会导致消息进入死信队列
        let _ = bus.publish(invalid);
    }

    // 执行状态工具，限制死信返回数量为2
    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "dead_letter_limit": 2
        }))
        .await
        .expect("tool execution should succeed");

    // 验证执行成功
    assert!(result.success);

    // 解析输出JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    // 验证死信队列的分页信息
    assert_eq!(parsed["dead_letter_count"], json!(3));
    assert_eq!(parsed["dead_letters_total"], json!(3));
    assert_eq!(parsed["dead_letter_offset"], json!(0));
    assert_eq!(parsed["dead_letters_returned"], json!(2));
    assert_eq!(parsed["dead_letters_truncated"], json!(true));
    assert_eq!(parsed["dead_letter_next_offset"], json!(2));
    assert_eq!(parsed["dead_letters"].as_array().map(Vec::len), Some(2));

    // 验证上下文信息（没有上下文）
    assert_eq!(parsed["contexts_total"], json!(0));
    assert_eq!(parsed["contexts_offset"], json!(0));
    assert_eq!(parsed["contexts_returned"], json!(0));
    assert_eq!(parsed["contexts_truncated"], json!(false));
    assert_eq!(parsed["context_next_offset"], serde_json::Value::Null);

    // 验证统计信息
    assert_eq!(parsed["stats"]["publish_attempts_total"], json!(0));
    assert_eq!(parsed["stats"]["deliveries_total"], json!(0));
    assert_eq!(parsed["stats"]["dead_letters_total"], json!(3));
    assert_eq!(parsed["stats"]["context_evictions_total"], json!(0));
    assert_eq!(parsed["stats"]["seen_message_id_evictions_total"], json!(0));
}

/// 测试状态工具对上下文条目的限制和最近访问顺序排序
///
/// # 测试场景
///
/// 1. 发布多个上下文补丁：
///    - patch_a (corr-a, phase=queued)
///    - patch_b (corr-b, phase=queued)
///    - patch_a_update (corr-a, phase=running) - 更新corr-a
///    - patch_c (corr-c, phase=queued)
/// 2. 使用 `context_limit: 2` 执行状态工具
/// 3. 验证返回的上下文按最近访问时间排序（LIFO顺序）
///
/// # 验证点
///
/// - 上下文总数为3
/// - 返回的上下文数为2（受limit限制）
/// - 返回的上下文顺序为：corr-c（最新）、corr-a（次新，因为被更新过）
/// - 下一页偏移量为2
/// - 第二页返回corr-a和corr-b（按更新时间排序）
#[tokio::test]
async fn status_tool_applies_context_limit_in_recent_order() {
    let bus = test_bus();

    // 创建并发布corr-a的初始状态补丁
    let mut patch_a = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-a",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-a/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch_a.correlation_id = Some("corr-a".to_string());
    bus.publish(patch_a).expect("patch a0 should publish");

    // 创建并发布corr-b的状态补丁
    let mut patch_b = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-b",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-b/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch_b.correlation_id = Some("corr-b".to_string());
    bus.publish(patch_b).expect("patch b0 should publish");

    // 更新corr-a的状态为running，这会使corr-a的访问时间更新
    let mut patch_a_update = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-a",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-a/state".to_string(),
            expected_version: 1,
            value: json!({"phase":"running"}),
        },
    );
    patch_a_update.correlation_id = Some("corr-a".to_string());
    bus.publish(patch_a_update).expect("patch a1 should publish");

    // 创建并发布corr-c的状态补丁（最新）
    let mut patch_c = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-c",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-c/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch_c.correlation_id = Some("corr-c".to_string());
    bus.publish(patch_c).expect("patch c0 should publish");

    // 执行状态工具，限制上下文返回数量为2，不包含死信
    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "context_limit": 2,
            "include_dead_letters": false
        }))
        .await
        .expect("tool execution should succeed");

    // 验证执行成功
    assert!(result.success);

    // 解析输出JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    // 验证上下文分页信息
    assert_eq!(parsed["context_count"], json!(3));
    assert_eq!(parsed["contexts_total"], json!(3));
    assert_eq!(parsed["contexts_offset"], json!(0));
    assert_eq!(parsed["contexts_returned"], json!(2));
    assert_eq!(parsed["contexts_truncated"], json!(true));
    assert_eq!(parsed["context_next_offset"], json!(2));

    // 验证死信队列（无死信）
    assert_eq!(parsed["dead_letters_total"], json!(0));
    assert_eq!(parsed["dead_letters_returned"], json!(0));
    assert_eq!(parsed["dead_letters_truncated"], json!(false));
    assert_eq!(parsed["dead_letter_next_offset"], serde_json::Value::Null);

    // 验证返回的上下文顺序：按最近访问时间排序
    // corr-c最新，corr-a次新（因为被更新过）
    assert_eq!(parsed["contexts"].as_array().map(Vec::len), Some(2));
    assert_eq!(parsed["contexts"][0]["key"], json!("delegate/corr-c/state"));
    assert_eq!(parsed["contexts"][1]["key"], json!("delegate/corr-a/state"));

    // 获取第二页数据，偏移量为1
    let second_page = tool
        .execute(json!({
            "context_limit": 2,
            "context_offset": 1,
            "include_dead_letters": false
        }))
        .await
        .expect("tool execution should succeed");

    // 验证第二页
    assert!(second_page.success);
    let second_parsed: serde_json::Value =
        serde_json::from_str(&second_page.output).expect("output must be valid JSON");

    // 验证第二页的分页信息
    assert_eq!(second_parsed["contexts_total"], json!(3));
    assert_eq!(second_parsed["contexts_offset"], json!(1));
    assert_eq!(second_parsed["contexts_returned"], json!(2));
    assert_eq!(second_parsed["contexts_truncated"], json!(false));
    assert_eq!(second_parsed["context_next_offset"], serde_json::Value::Null);

    // 第二页返回：corr-a和corr-b
    assert_eq!(second_parsed["contexts"][0]["key"], json!("delegate/corr-a/state"));
    assert_eq!(second_parsed["contexts"][1]["key"], json!("delegate/corr-b/state"));
}

/// 测试状态工具对上下文的分页和关联ID过滤功能
///
/// # 测试场景
///
/// 1. 发布多个上下文补丁：
///    - corr-a 的 state 补丁
///    - corr-b 的 state 补丁
///    - corr-a 的 output 补丁
/// 2. 使用关联ID过滤 `correlation_id: "corr-a"` 和分页参数
/// 3. 验证只返回corr-a相关的上下文
///
/// # 验证点
///
/// - 上下文总数为2（corr-a有2个上下文条目）
/// - 使用偏移量1和限制1，返回第二个条目
/// - 返回的上下文键为 `delegate/corr-a/state`
/// - 没有下一页（已到达末尾）
#[tokio::test]
async fn status_tool_applies_context_paging_with_correlation_filter() {
    let bus = test_bus();

    // 创建并发布corr-a的状态补丁
    let mut patch_a_state = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-a",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-a/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch_a_state.correlation_id = Some("corr-a".to_string());
    bus.publish(patch_a_state).expect("corr-a state patch should publish");

    // 创建并发布corr-b的状态补丁
    let mut patch_b_state = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-b",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-b/state".to_string(),
            expected_version: 0,
            value: json!({"phase":"queued"}),
        },
    );
    patch_b_state.correlation_id = Some("corr-b".to_string());
    bus.publish(patch_b_state).expect("corr-b state patch should publish");

    // 创建并发布corr-a的输出补丁
    let mut patch_a_output = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-a",
        "delegate.output",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-a/output".to_string(),
            expected_version: 0,
            value: json!({"summary":"ready"}),
        },
    );
    patch_a_output.correlation_id = Some("corr-a".to_string());
    bus.publish(patch_a_output).expect("corr-a output patch should publish");

    // 执行状态工具，使用关联ID过滤和分页参数
    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "correlation_id": "corr-a",
            "context_limit": 1,
            "context_offset": 1,
            "include_dead_letters": false
        }))
        .await
        .expect("tool execution should succeed");

    // 验证执行成功
    assert!(result.success);

    // 解析输出JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    // 验证上下文分页和过滤结果
    // corr-a有2个上下文条目：state和output
    assert_eq!(parsed["contexts_total"], json!(2));
    assert_eq!(parsed["contexts_offset"], json!(1));
    assert_eq!(parsed["contexts_returned"], json!(1));
    assert_eq!(parsed["contexts_truncated"], json!(false));
    assert_eq!(parsed["context_next_offset"], serde_json::Value::Null);

    // 验证返回的是第二个上下文条目（state）
    assert_eq!(parsed["contexts"][0]["key"], json!("delegate/corr-a/state"));
}

/// 测试状态工具对死信队列的分页和关联ID过滤功能
///
/// # 测试场景
///
/// 1. 发布3条无效消息到死信队列：
///    - dead-corr-0 (corr-a) - 发送到未知代理
///    - dead-corr-1 (corr-b) - 发送到未知代理
///    - dead-corr-2 (corr-a) - 发送到未知代理
/// 2. 使用关联ID过滤 `correlation_id: "corr-a"` 和分页参数
/// 3. 验证分页正确返回corr-a相关的死信
///
/// # 验证点
///
/// - 第一页：返回1条死信（dead-corr-2），总数2，有下一页
/// - 第二页：返回1条死信（dead-corr-0），无下一页
/// - 死信按最新优先顺序返回
#[tokio::test]
async fn status_tool_applies_dead_letter_paging_with_correlation_filter() {
    let bus = test_bus();

    // 发布3条消息到死信队列：目标是未知代理
    // 其中2条属于corr-a，1条属于corr-b
    for (index, correlation_id) in [("0", "corr-a"), ("1", "corr-b"), ("2", "corr-a")] {
        let mut invalid = CoordinationEnvelope::new_direct(
            "delegate-lead",
            "unknown-agent", // 目标代理不存在，消息会进入死信队列
            format!("delegate:{correlation_id}"),
            "delegate.request",
            CoordinationPayload::DelegateTask {
                task_id: format!("task-{index}"),
                summary: "invalid target".to_string(),
                metadata: json!({}),
            },
        );
        invalid.id = format!("dead-corr-{index}");
        invalid.correlation_id = Some(correlation_id.to_string());
        let _ = bus.publish(invalid);
    }

    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));

    // 获取第一页：限制1条，偏移0
    let first_page = tool
        .execute(json!({
            "correlation_id": "corr-a",
            "dead_letter_limit": 1,
            "dead_letter_offset": 0
        }))
        .await
        .expect("tool execution should succeed");

    // 验证第一页
    assert!(first_page.success);
    let first_parsed: serde_json::Value =
        serde_json::from_str(&first_page.output).expect("output must be valid JSON");

    // corr-a相关的死信总数为2
    assert_eq!(first_parsed["dead_letters_total"], json!(2));
    assert_eq!(first_parsed["dead_letter_offset"], json!(0));
    assert_eq!(first_parsed["dead_letters_returned"], json!(1));
    assert_eq!(first_parsed["dead_letters_truncated"], json!(true));
    assert_eq!(first_parsed["dead_letter_next_offset"], json!(1));

    // 第一页返回最新的死信（dead-corr-2）
    assert_eq!(first_parsed["dead_letters"][0]["message_id"], json!("dead-corr-2"));

    // 获取第二页：限制1条，偏移1
    let second_page = tool
        .execute(json!({
            "correlation_id": "corr-a",
            "dead_letter_limit": 1,
            "dead_letter_offset": 1
        }))
        .await
        .expect("tool execution should succeed");

    // 验证第二页
    assert!(second_page.success);
    let second_parsed: serde_json::Value =
        serde_json::from_str(&second_page.output).expect("output must be valid JSON");

    // 第二页的分页信息
    assert_eq!(second_parsed["dead_letters_total"], json!(2));
    assert_eq!(second_parsed["dead_letter_offset"], json!(1));
    assert_eq!(second_parsed["dead_letters_returned"], json!(1));
    assert_eq!(second_parsed["dead_letters_truncated"], json!(false));
    assert_eq!(second_parsed["dead_letter_next_offset"], serde_json::Value::Null);

    // 第二页返回较早的死信（dead-corr-0）
    assert_eq!(second_parsed["dead_letters"][0]["message_id"], json!("dead-corr-0"));
}

/// 测试状态工具对收件箱消息的分页和关联ID过滤功能
///
/// # 测试场景
///
/// 1. 发布4条消息到researcher的收件箱：
///    - msg-corr-0 (corr-a)
///    - msg-corr-1 (corr-b)
///    - msg-corr-2 (corr-a)
///    - msg-corr-3 (corr-a)
/// 2. 使用关联ID过滤 `correlation_id: "corr-a"` 和分页参数
/// 3. 验证分页正确返回corr-a相关的消息
///
/// # 验证点
///
/// - 收件箱总消息数为4
/// - 过滤后（corr-a）的消息数为3
/// - 第一页（偏移1，限制1）：返回msg-corr-2，有下一页
/// - 第二页（偏移2，限制1）：返回msg-corr-3，无下一页
/// - 消息按时间倒序返回
#[tokio::test]
async fn status_tool_applies_message_paging_with_correlation_filter() {
    let bus = test_bus();

    // 发布4条消息：3条属于corr-a，1条属于corr-b
    for (message_id, correlation_id) in [
        ("msg-corr-0", "corr-a"),
        ("msg-corr-1", "corr-b"),
        ("msg-corr-2", "corr-a"),
        ("msg-corr-3", "corr-a"),
    ] {
        let mut request = CoordinationEnvelope::new_direct(
            "delegate-lead",
            "researcher",
            format!("delegate:{correlation_id}"),
            "delegate.request",
            CoordinationPayload::DelegateTask {
                task_id: message_id.to_string(),
                summary: "Investigate".to_string(),
                metadata: json!({"priority":"high"}),
            },
        );
        request.id = message_id.to_string();
        request.correlation_id = Some(correlation_id.to_string());
        bus.publish(request).expect("request should publish");
    }

    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));

    // 获取第一页：偏移1，限制1，过滤corr-a的消息
    let first_page = tool
        .execute(json!({
            "agent": "researcher",
            "correlation_id": "corr-a",
            "include_messages": true,
            "message_limit": 1,
            "message_offset": 1,
            "include_dead_letters": false
        }))
        .await
        .expect("tool execution should succeed");

    // 验证第一页
    assert!(first_page.success);
    let first_parsed: serde_json::Value =
        serde_json::from_str(&first_page.output).expect("output must be valid JSON");

    // 验证收件箱信息
    assert_eq!(first_parsed["inboxes"].as_array().map(Vec::len), Some(1));
    assert_eq!(first_parsed["inboxes"][0]["pending"], json!(4)); // 总待处理消息
    assert_eq!(first_parsed["inboxes"][0]["pending_filtered"], json!(3)); // 过滤后的待处理消息
    assert_eq!(first_parsed["inboxes"][0]["message_total"], json!(3)); // 过滤后的消息总数
    assert_eq!(first_parsed["inboxes"][0]["message_offset"], json!(1));
    assert_eq!(first_parsed["inboxes"][0]["messages_returned"], json!(1));
    assert_eq!(first_parsed["inboxes"][0]["messages_truncated"], json!(true));
    assert_eq!(first_parsed["inboxes"][0]["message_next_offset"], json!(2));

    // 第一页返回第二个corr-a消息（msg-corr-2）
    assert_eq!(first_parsed["inboxes"][0]["messages"][0]["message_id"], json!("msg-corr-2"));

    // 获取第二页：偏移2，限制1
    let second_page = tool
        .execute(json!({
            "agent": "researcher",
            "correlation_id": "corr-a",
            "include_messages": true,
            "message_limit": 1,
            "message_offset": 2,
            "include_dead_letters": false
        }))
        .await
        .expect("tool execution should succeed");

    // 验证第二页
    assert!(second_page.success);
    let second_parsed: serde_json::Value =
        serde_json::from_str(&second_page.output).expect("output must be valid JSON");

    // 验证第二页的分页信息
    assert_eq!(second_parsed["inboxes"][0]["message_total"], json!(3));
    assert_eq!(second_parsed["inboxes"][0]["message_offset"], json!(2));
    assert_eq!(second_parsed["inboxes"][0]["messages_returned"], json!(1));
    assert_eq!(second_parsed["inboxes"][0]["messages_truncated"], json!(false));
    assert_eq!(second_parsed["inboxes"][0]["message_next_offset"], serde_json::Value::Null);

    // 第二页返回第三个corr-a消息（msg-corr-3）
    assert_eq!(second_parsed["inboxes"][0]["messages"][0]["message_id"], json!("msg-corr-3"));
}

#[test]
fn status_tool_metadata_and_schema_describe_supported_arguments() {
    let tool = DelegateCoordinationStatusTool::new(test_bus(), Arc::new(SecurityPolicy::default()));

    assert_eq!(tool.name(), "delegate_coordination_status");
    assert!(tool.description().contains("委托协调运行时状态"));

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], json!("object"));
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(schema["required"], json!([]));
    assert_eq!(schema["properties"]["include_messages"]["default"], json!(false));
    assert_eq!(schema["properties"]["include_dead_letters"]["default"], json!(true));
    assert_eq!(schema["properties"]["message_limit"]["default"], json!(5));
    assert_eq!(schema["properties"]["message_limit"]["maximum"], json!(50));
    assert_eq!(schema["properties"]["dead_letter_limit"]["default"], json!(10));
    assert_eq!(schema["properties"]["dead_letter_limit"]["maximum"], json!(100));
    assert_eq!(schema["properties"]["context_limit"]["default"], json!(25));
    assert_eq!(schema["properties"]["context_limit"]["maximum"], json!(200));
    assert_eq!(schema["properties"]["message_offset"]["maximum"], json!(10_000));
    assert_eq!(schema["properties"]["dead_letter_offset"]["maximum"], json!(10_000));
    assert_eq!(schema["properties"]["context_offset"]["maximum"], json!(10_000));
}

#[test]
fn clamp_helpers_apply_defaults_and_bounds() {
    assert_eq!(super::clamp_usize(None, 5, 50), 5);
    assert_eq!(super::clamp_usize(Some(0), 5, 50), 5);
    assert_eq!(super::clamp_usize(Some(7), 5, 50), 7);
    assert_eq!(super::clamp_usize(Some(70), 5, 50), 50);

    assert_eq!(super::clamp_offset(None, 10_000), 0);
    assert_eq!(super::clamp_offset(Some(0), 10_000), 0);
    assert_eq!(super::clamp_offset(Some(37), 10_000), 37);
    assert_eq!(super::clamp_offset(Some(20_000), 10_000), 10_000);
}

#[test]
fn payload_kind_identifies_every_payload_variant() {
    let cases = [
        (
            CoordinationPayload::DelegateTask {
                task_id: "task".to_string(),
                summary: "summary".to_string(),
                metadata: json!({}),
            },
            "delegate_task",
        ),
        (
            CoordinationPayload::ContextPatch {
                key: "delegate/corr/state".to_string(),
                expected_version: 0,
                value: json!({"phase": "queued"}),
            },
            "context_patch",
        ),
        (
            CoordinationPayload::TaskResult {
                task_id: "task".to_string(),
                success: true,
                output: "done".to_string(),
            },
            "task_result",
        ),
        (CoordinationPayload::Ack { acked_message_id: "message".to_string() }, "ack"),
        (
            CoordinationPayload::Control {
                action: "pause".to_string(),
                note: Some("maintenance".to_string()),
            },
            "control",
        ),
    ];

    for (payload, expected) in cases {
        assert_eq!(super::payload_kind(&payload), expected);
    }
}

#[test]
fn summarize_envelope_reports_transport_fields_and_payload_kind() {
    let mut envelope = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "researcher",
        "delegate:corr-summary",
        "delegate.ack",
        CoordinationPayload::Ack { acked_message_id: "request-1".to_string() },
    );
    envelope.id = "ack-1".to_string();
    envelope.correlation_id = Some("corr-summary".to_string());
    envelope.causation_id = Some("request-1".to_string());

    let summary = super::summarize_envelope(SequencedEnvelope { sequence: 42, envelope });

    assert_eq!(
        summary,
        json!({
            "sequence": 42,
            "message_id": "ack-1",
            "topic": "delegate.ack",
            "from": "delegate-lead",
            "to": "researcher",
            "correlation_id": "corr-summary",
            "causation_id": "request-1",
            "payload_kind": "ack"
        })
    );
}

#[tokio::test]
async fn status_tool_ignores_blank_filters_and_uses_default_message_limit() {
    let bus = test_bus();
    for index in 0..6 {
        let mut request = CoordinationEnvelope::new_direct(
            "delegate-lead",
            "researcher",
            "delegate:corr-defaults",
            "delegate.request",
            CoordinationPayload::DelegateTask {
                task_id: format!("task-{index}"),
                summary: "Investigate".to_string(),
                metadata: json!({}),
            },
        );
        request.id = format!("default-message-{index}");
        request.correlation_id = Some("corr-defaults".to_string());
        bus.publish(request).expect("request should publish");
    }

    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "agent": "   ",
            "correlation_id": "\t ",
            "include_messages": true,
            "message_limit": 0
        }))
        .await
        .expect("tool execution should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    assert!(result.success);
    assert_eq!(parsed["filter"]["agent"], serde_json::Value::Null);
    assert_eq!(parsed["filter"]["correlation_id"], serde_json::Value::Null);
    assert_eq!(parsed["inboxes"].as_array().map(Vec::len), Some(2));
    let researcher = parsed["inboxes"]
        .as_array()
        .and_then(|inboxes| {
            inboxes.iter().find(|inbox| inbox["agent"].as_str() == Some("researcher"))
        })
        .expect("researcher inbox should be present");
    assert_eq!(researcher["pending"], json!(6));
    assert_eq!(researcher["message_total"], json!(6));
    assert_eq!(researcher["messages_returned"], json!(5));
    assert_eq!(researcher["messages_truncated"], json!(true));
    assert_eq!(researcher["message_next_offset"], json!(5));
}

#[tokio::test]
async fn status_tool_skips_an_unknown_agent() {
    let tool = DelegateCoordinationStatusTool::new(test_bus(), Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({"agent": "missing-agent", "include_messages": true}))
        .await
        .expect("tool execution should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    assert!(result.success);
    assert_eq!(parsed["filter"]["agent"], json!("missing-agent"));
    assert_eq!(parsed["inboxes"], json!([]));
}

#[tokio::test]
async fn status_tool_hides_dead_letters_when_preview_is_disabled() {
    let bus = test_bus();
    let mut invalid = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "missing-agent",
        "delegate:corr-hidden",
        "delegate.request",
        CoordinationPayload::DelegateTask {
            task_id: "corr-hidden".to_string(),
            summary: "invalid target".to_string(),
            metadata: json!({}),
        },
    );
    invalid.id = "hidden-dead-letter".to_string();
    invalid.correlation_id = Some("corr-hidden".to_string());
    let _ = bus.publish(invalid);

    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({"include_dead_letters": false}))
        .await
        .expect("tool execution should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    assert!(result.success);
    assert_eq!(parsed["dead_letter_count"], json!(1));
    assert_eq!(parsed["dead_letters_total"], json!(0));
    assert_eq!(parsed["dead_letters_returned"], json!(0));
    assert_eq!(parsed["dead_letters_truncated"], json!(false));
    assert_eq!(parsed["dead_letter_next_offset"], serde_json::Value::Null);
    assert_eq!(parsed["dead_letters"], json!([]));
}

#[tokio::test]
async fn status_tool_returns_empty_pages_when_offsets_exceed_totals() {
    let bus = test_bus();

    let mut request = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "researcher",
        "delegate:corr-edge",
        "delegate.request",
        CoordinationPayload::DelegateTask {
            task_id: "corr-edge".to_string(),
            summary: "Investigate".to_string(),
            metadata: json!({}),
        },
    );
    request.id = "edge-message".to_string();
    request.correlation_id = Some("corr-edge".to_string());
    bus.publish(request).expect("request should publish");

    let mut patch = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "delegate-lead",
        "delegate:corr-edge",
        "delegate.state",
        CoordinationPayload::ContextPatch {
            key: "delegate/corr-edge/state".to_string(),
            expected_version: 0,
            value: json!({"phase": "queued"}),
        },
    );
    patch.id = "edge-context".to_string();
    patch.correlation_id = Some("corr-edge".to_string());
    bus.publish(patch).expect("context patch should publish");

    let mut invalid = CoordinationEnvelope::new_direct(
        "delegate-lead",
        "missing-agent",
        "delegate:corr-edge",
        "delegate.request",
        CoordinationPayload::DelegateTask {
            task_id: "corr-edge-invalid".to_string(),
            summary: "invalid target".to_string(),
            metadata: json!({}),
        },
    );
    invalid.id = "edge-dead-letter".to_string();
    invalid.correlation_id = Some("corr-edge".to_string());
    let _ = bus.publish(invalid);

    let tool = DelegateCoordinationStatusTool::new(bus, Arc::new(SecurityPolicy::default()));
    let result = tool
        .execute(json!({
            "agent": "researcher",
            "correlation_id": "corr-edge",
            "include_messages": true,
            "message_limit": 1,
            "message_offset": 10,
            "dead_letter_limit": 1,
            "dead_letter_offset": 10,
            "context_limit": 1,
            "context_offset": 10
        }))
        .await
        .expect("tool execution should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.output).expect("output must be valid JSON");

    assert!(result.success);
    assert_eq!(parsed["inboxes"][0]["message_total"], json!(1));
    assert_eq!(parsed["inboxes"][0]["messages_returned"], json!(0));
    assert_eq!(parsed["inboxes"][0]["messages_truncated"], json!(false));
    assert_eq!(parsed["inboxes"][0]["message_next_offset"], serde_json::Value::Null);
    assert_eq!(parsed["contexts_total"], json!(1));
    assert_eq!(parsed["contexts_returned"], json!(0));
    assert_eq!(parsed["contexts_truncated"], json!(false));
    assert_eq!(parsed["context_next_offset"], serde_json::Value::Null);
    assert_eq!(parsed["dead_letters_total"], json!(1));
    assert_eq!(parsed["dead_letters_returned"], json!(0));
    assert_eq!(parsed["dead_letters_truncated"], json!(false));
    assert_eq!(parsed["dead_letter_next_offset"], serde_json::Value::Null);
}
