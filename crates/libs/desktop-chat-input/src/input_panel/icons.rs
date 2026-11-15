use iced::widget::svg;
use iced::{Length, Theme};
use desktop_assets::assets::{self, Icon};
pub fn provider_logo_handle(provider_id: &str) -> svg::Handle {
    // crate 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    assets::get_provider_icon(provider_id)
}
pub fn auto_icon() -> svg::Handle {
    // assets 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    assets::get_icon(Icon::Star)
}
pub fn icon_svg<'a>(icon: Icon, size: f32) -> svg::Svg<'a> {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Svg::<'a, iced::Theme>::new(assets::get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}
pub fn themed_svg_handle<'a>(handle: svg::Handle, size: f32) -> svg::Svg<'a> {
    // svg 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    svg::Svg::<'a, iced::Theme>::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().text) })
}
