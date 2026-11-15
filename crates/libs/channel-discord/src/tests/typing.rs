use super::*;

/// 测试正在输入句柄初始状态为空
/// 新创建的频道应该有空的正在输入句柄映射
#[test]
fn typing_handles_start_empty() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    let guard = ch.typing_handles.lock();
    assert!(guard.is_empty());
}

/// 测试启动正在输入状态会设置句柄
/// 调用 start_typing 后，句柄映射应该包含对应的条目
#[tokio::test]
async fn start_typing_sets_handle() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    let _ = ch.start_typing("123456").await;
    let guard = ch.typing_handles.lock();
    assert!(guard.contains_key("123456"));
}

/// 测试停止正在输入状态会清除句柄
/// 调用 stop_typing 后，句柄映射应该移除对应的条目
#[tokio::test]
async fn stop_typing_clears_handle() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    let _ = ch.start_typing("123456").await;
    let _ = ch.stop_typing("123456").await;
    let guard = ch.typing_handles.lock();
    assert!(!guard.contains_key("123456"));
}

/// 测试停止正在输入状态的幂等性
/// 多次调用 stop_typing 应该都是安全的
#[tokio::test]
async fn stop_typing_is_idempotent() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    assert!(ch.stop_typing("123456").await.is_ok());
    assert!(ch.stop_typing("123456").await.is_ok());
}

/// 测试多个正在输入句柄的并发独立性
/// 不同用户/频道的正在输入状态应该互不影响
#[tokio::test]
async fn concurrent_typing_handles_are_independent() {
    let ch = DiscordChannel::new("fake".into(), None, vec![], false, false);
    let _ = ch.start_typing("111").await;
    let _ = ch.start_typing("222").await;
    {
        let guard = ch.typing_handles.lock();
        assert_eq!(guard.len(), 2);
        assert!(guard.contains_key("111"));
        assert!(guard.contains_key("222"));
    }
    let _ = ch.stop_typing("111").await;
    let guard = ch.typing_handles.lock();
    assert_eq!(guard.len(), 1);
    assert!(guard.contains_key("222"));
}
