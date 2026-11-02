//! Git presentation retains ranges and raw paths as semantic values.
use super::I18n;
pub(crate) fn comment_range(i18n: I18n, start: usize, end: usize, old: bool) -> String {
    let side = i18n.__(if old { "旧行" } else { "新行" });
    if start == end {
        i18n.format("正在评论 %s %u", &[side.into(), (start + 1).into()])
    } else {
        i18n.format("正在评论 %s %u-%u", &[side.into(), (start + 1).into(), (end + 1).into()])
    }
    .expect("valid comment range")
}
