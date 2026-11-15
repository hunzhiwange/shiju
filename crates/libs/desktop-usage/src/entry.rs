use iced::Element;
#[derive(Clone, Debug)]
pub enum Navigation { Auth, Apps }
pub fn external_usage_entry() -> Element<'static, Navigation> {
    use iced::widget::{button, column, container, row, text};

    use desktop_ui_controls::system_settings_common::{
        rounded_action_btn_style, settings_muted_text_style,
    };


    let open = |label: &'static str, tab| {
        button(text(label).size(13))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press(tab)
    };
    container(column![
        text("应用额度与套餐").size(16),
        text("连接执行主机后，在授权账号中读取额度；供应商用量位于应用的供应商卡片中。查询结果与当前会话统计分别展示。")
            .size(13)
            .style(settings_muted_text_style),
        row![
            open("授权账号额度", Navigation::Auth),
            open("供应商用量", Navigation::Apps),
        ].spacing(8),
    ].spacing(8))
    .padding([12, 18])
    .width(iced::Length::Fill)
    .into()
}
