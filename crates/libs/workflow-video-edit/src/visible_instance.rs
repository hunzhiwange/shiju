//! Visible-instance source contract, independent of native skill registry types.
use std::path::{Path, PathBuf};
pub fn visible_shiju_root<'a>(locations: impl IntoIterator<Item = Option<&'a Path>>) -> Result<PathBuf, String> {
    let mut matching = locations.into_iter();
    let location = matching.next().ok_or("当前会话没有授权的 shiju 技能")?;
    if matching.next().is_some() {
        return Err("当前会话有多个 shiju 实例，请先明确选择".into());
    }
    let location = location.ok_or("shiju 技能缺少客户端来源路径")?;
    let file = location.canonicalize().map_err(|_| "shiju 技能路径不可用")?;
    if !file.is_file() || file.file_name().and_then(|s| s.to_str()) != Some("SKILL.md") {
        return Err("shiju 来源必须是当前加载的 SKILL.md".into());
    }
    Ok(file.parent().ok_or("shiju 技能缺少包目录")?.to_path_buf())
}
