//! 验证原生浏览器快照脚本的选项嵌入与页面采集契约。
//! 脚本字符串测试固定交互过滤、紧凑模式和深度上限的 JavaScript 合同。

use super::*;

#[test]
fn script_embeds_boolean_flags_and_depth() {
    let script = snapshot_script(true, false, Some(3));
    assert!(script.contains("const interactiveOnly = true;"));
    assert!(script.contains("const compact = false;"));
    assert!(script.contains("const maxDepth = 3;"));
}

#[test]
fn script_uses_null_depth_when_unbounded() {
    let script = snapshot_script(false, true, None);
    assert!(script.contains("const interactiveOnly = false;"));
    assert!(script.contains("const compact = true;"));
    assert!(script.contains("const maxDepth = null;"));
}

#[test]
fn script_contains_dom_collection_contract() {
    let script = snapshot_script(true, true, Some(2));

    assert!(script.starts_with("(() => {"));
    assert!(script.contains("const nodes = [];"));
    assert!(script.contains("data-zc-ref"));
    assert!(script.contains("nodes.length >= 400"));
    assert!(script.contains("title: document.title"));
    assert!(script.contains("url: window.location.href"));
    assert!(script.ends_with(")();"));
}
