//! Feedback templates use the shared safe formatter.
use vw_config_types::ui::DesktopLocale;
pub fn render(locale: DesktopLocale, source: &'static str, args: &[String]) -> String {
    let args =
        args.iter().cloned().map(crate::format::Argument::from).collect::<Vec<_>>();
    crate::I18n::new(locale)
        .format(source, &args)
        .expect("valid settings feedback template")
}
