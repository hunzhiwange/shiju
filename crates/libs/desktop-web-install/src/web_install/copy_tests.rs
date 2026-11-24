use super::{ProductContext as _, message};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;

#[test]
fn localized_web_error_keeps_opaque_causes_and_rerenders() {
    let raw = "raw/path/%s/%n/中文";
    let result: anyhow::Result<()> = Err(anyhow::Error::msg(raw))
        .product_context("无法获取用户目录");
    let copy = message(&result.unwrap_err());
    assert_eq!(copy.render(I18n::new(DesktopLocale::English)).unwrap(),
        format!("Cannot determine user directory: {raw}"));
    assert_eq!(copy.render(I18n::new(DesktopLocale::SimplifiedChinese)).unwrap(),
        format!("无法获取用户目录: {raw}"));
}
