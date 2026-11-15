//! Worktree picker policy; the desktop executes session, preference and gateway effects.
pub fn order_options(list: &mut Vec<(String, String)>, last_directory: Option<&String>) {
    if let Some(last_directory) = last_directory
        && let Some(pos) = list.iter().position(|(directory, _)| directory == last_directory)
    {
        let item = list.remove(pos);
        let insert_at = if !list.is_empty() && list[0].1 == "主工作区" { 1 } else { 0 };
        list.insert(insert_at, item);
    }
}
pub fn requested_name(value: &str) -> Result<String, &'static str> {
    let requested_name = value.trim().to_string();
    if requested_name.is_empty() { return Err("请输入 worktree 英文名称"); }
    if !requested_name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        return Err("worktree 名称仅支持小写英文、数字、连字符(-)");
    }
    Ok(requested_name)
}
pub fn is_primary(options: &[(String, String)], project: Option<&str>, directory: &str) -> bool {
    options.iter().any(|(d, label)| d == directory && label == "主工作区")
        || directory == project.unwrap_or_default()
}
