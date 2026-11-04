//! ACP 卡片与配置弹窗共用的连接测试状态。

use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::app::Message;
use crate::app::components::status_animation::spinner_frame;
use crate::app::state::AcpDiagnosticStatus;

fn status_color(theme: &Theme, status: AcpDiagnosticStatus) -> Color {
    let palette = theme.extended_palette();
    match status {
        AcpDiagnosticStatus::Connected => palette.success.base.color,
        AcpDiagnosticStatus::Running | AcpDiagnosticStatus::AuthenticationRequired => {
            palette.warning.base.color
        }
        AcpDiagnosticStatus::Cancelled => theme.palette().text.scale_alpha(0.65),
        _ => palette.danger.base.color,
    }
}

pub(super) fn diagnostic_status(
    status: AcpDiagnosticStatus,
    frame: usize,
) -> Element<'static, Message> {
    let (icon, label) = match status {
        AcpDiagnosticStatus::Running => (spinner_frame(frame), "测试中"),
        AcpDiagnosticStatus::Connected => ("✓", "测试成功"),
        AcpDiagnosticStatus::AuthenticationRequired => ("!", "需要认证"),
        AcpDiagnosticStatus::Cancelled => ("■", "已取消"),
        _ => ("×", "测试失败"),
    };
    container(
        row![
            container(text(icon).size(18).style(move |theme: &Theme| {
                iced::widget::text::Style { color: Some(status_color(theme, status)) }
            }))
            .width(24)
            .center_x(24),
            text(label).size(13),
            text(status.description()).size(12).width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([9, 12])
    .width(Length::Fill)
    .style(move |theme: &Theme| {
        let color = status_color(theme, status);
        container::Style {
            text_color: Some(theme.palette().text),
            background: Some(color.scale_alpha(0.10).into()),
            border: iced::Border { color: color.scale_alpha(0.55), width: 1.0, radius: 8.0.into() },
            ..Default::default()
        }
    })
    .into()
}
