//! Settings feedback produced locally; unknown diagnostics pass through unchanged.
use super::I18n;

pub fn message(i18n: I18n, raw: &str) -> String {
    for (prefix, suffix, source) in [
        ("电脑操控请求失败：", "。请刷新后重试。", "电脑操控请求失败：%s。请刷新后重试。"),
        ("申请系统权限失败：", "", "申请系统权限失败：%s"),
        ("本次运行已停用全局热键，但停用设置保存失败：", "", "本次运行已停用全局热键，但停用设置保存失败：%s"),
        ("", " 保存将重置本机偏好。", "%s 保存将重置本机偏好。"),
        ("清理结果可能不完整，请刷新分类核对：", "", "清理结果可能不完整，请刷新分类核对：%s"),
        ("删除失败（可能存在保护或版本冲突）；请刷新后核对：", "", "删除失败（可能存在保护或版本冲突）；请刷新后核对：%s"),
        ("列表项已存在：", "", "列表项已存在：%s"),
        ("系统设置未保存：", "", "系统设置未保存：%s"),
    ] {
        if let Some(value) = raw.strip_prefix(prefix).and_then(|value| value.strip_suffix(suffix)) {
            return crate::__!(i18n, source, value).expect("valid settings feedback");
        }
    }
    if let Some(value) = raw.strip_prefix("全局热键已启用：听写 ").and_then(|value| value.strip_suffix('。')) {
        if let Some((dictation, chat)) = value.split_once("；语音聊天 ") {
            return crate::__!(i18n, "全局热键已启用：听写 %s；语音聊天 %s。", dictation, chat).expect("valid hotkey status");
        }
    }
    if let Some(value) = raw.strip_prefix("投递通道 ").and_then(|value| value.strip_suffix(" 未配置；请先在通道设置中配置凭证")) {
        return crate::__!(i18n, "投递通道 %s 未配置；请先在通道设置中配置凭证", value).expect("valid delivery feedback");
    }
    if let Some(value) = raw.strip_prefix("部分清理：已释放 ").and_then(|value| value.strip_suffix(" 项未删除。请核对保护规则后刷新。")) {
        if let Some((bytes, count)) = value.split_once(" 字节，") {
            if let (Ok(bytes), Ok(count)) = (bytes.parse::<u64>(), count.parse::<u64>()) {
                return crate::__!(i18n, "部分清理：已释放 %u 字节，%u 项未删除。请核对保护规则后刷新。", bytes, count).expect("valid cleanup feedback");
            }
        }
    }
    if let Some(bytes) = raw.strip_prefix("清理完成：已释放 ").and_then(|value| value.strip_suffix(" 字节。")) {
        if let Ok(bytes) = bytes.parse::<u64>() {
            return crate::__!(i18n, "清理完成：已释放 %u 字节。", bytes).expect("valid cleanup feedback");
        }
    }
    if let Some(value) = raw.strip_prefix("加载主题 `") {
        if let Some((id, diagnostic)) = value.split_once("` 失败: ") {
            return crate::__!(i18n, "加载主题 `%s` 失败: %s", id, diagnostic).expect("valid theme feedback");
        }
    }
    if let Some(id) = raw.strip_prefix("无法定位主题 `").and_then(|value| value.strip_suffix("` 的目录")) {
        return crate::__!(i18n, "无法定位主题 `%s` 的目录", id).expect("valid theme directory feedback");
    }
    // Account storage errors contain an untouched path followed by a known local reason.
    if let Some((path, reason)) = raw.rsplit_once('：') {
        if matches!(reason,
            "无法读取账户文件" | "账户文件格式无效" | "账户身份字段无效" |
            "账户资料字段无效" | "账户凭据为空" | "账户 ID 重复或无效" |
            "当前账户 ID 不存在" | "无法创建账户目录" | "无法写入账户文件" |
            "无法保存账户文件" | "无法限制账户文件权限" | "无法编码账户文件") {
            return format!("{path}：{}", i18n.__(reason));
        }
    }
    i18n.__(raw).to_owned()
}
