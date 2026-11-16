// 宿主仅保留成功载荷写入活动会话缓存及索引、无关 ViewMessage 路由契约；其余状态行为在 desktop-usage 独立测试。
//! 用量面板消息处理测试，覆盖模型限额信息和步骤原始用量的加载状态。
//! 重点验证展开、缓存与会话切换不会触发重复请求或遗留旧弹窗。

use crate::app::{App, message::ViewMessage};

fn test_app() -> App {
    App::new().0
}

fn raw_key(session_id: &str, step_index: u32) -> (String, String, u32) {
    (session_id.to_string(), format!("legacy-step-{step_index}"), step_index)
}

#[test]
fn usage_step_raw_loaded_for_active_session_caches_success_and_error() {
    let cases = [
        Ok(Some(serde_json::json!({ "request": "cached" }))),
        Err("payload unavailable".to_string()),
    ];

    for (step_index, result) in cases.into_iter().enumerate() {
        let mut app = test_app();
        let key = raw_key("active-session", step_index as u32);
        app.active_session_id = Some(key.0.clone());
        app.usage_panel.usage_step_raw_loading.insert(key.clone());

        let _task = super::update(
            &mut app,
            ViewMessage::UsageStepRawLoaded(key.0.clone(), key.2, result.clone()),
        );

        assert!(!app.usage_panel.usage_step_raw_loading.contains(&key));
        if let Ok(Some(raw)) = result {
            assert_eq!(app.active_session_view_state.step_llm_raw.get(&key), Some(&raw));
            assert!(app.active_session_view_state.llm_raw_step_indices.contains(&key));
            assert!(!app.usage_panel.usage_step_raw_payloads.contains_key(&key));
        } else {
            assert_eq!(app.usage_panel.usage_step_raw_payloads.get(&key), Some(&result));
        }
    }
}

#[test]
fn unrelated_message_preserves_usage_panel_state() {
    let mut app = test_app();
    let key = raw_key("active-session", 17);
    app.active_session_id = Some(key.0.clone());
    app.usage_panel.usage_step_expanded.insert(key.2);
    app.usage_panel.usage_step_raw_loading.insert(key.clone());
    app.usage_panel.usage_step_raw_modal =
        Some(crate::app::state::UsageStepRawModal { step_index: key.2, content: "{}".to_string() });

    let _task = super::update(&mut app, ViewMessage::OpenUsage);

    assert!(app.usage_panel.usage_step_expanded.contains(&key.2));
    assert!(app.usage_panel.usage_step_raw_loading.contains(&key));
    assert!(app.usage_panel.usage_step_raw_modal.is_some());
}
