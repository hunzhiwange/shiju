use super::*;
use tui_contracts::action::{V3UserInputOption, V3UserInputQuestion, V3UserInputRequest};

fn text(buf: &Buffer, area: Rect) -> String {
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// 验证 `request_user_input_render_preserves_chinese_options_and_focus_hint` 场景的触发条件与预期结果。
#[test]
fn request_user_input_render_preserves_chinese_options_and_focus_hint() {
    let request = V3UserInputRequest::new(
        "render-1",
        vec![V3UserInputQuestion::new(
            "mode",
            "模式",
            "请选择工作方式",
            vec![
                V3UserInputOption::new("安全", "只读检查"),
                V3UserInputOption::new("快速", "允许更多操作"),
            ],
        )],
    );
    let overlay = RequestUserInputOverlay::new(request);
    let area = Rect::new(0, 0, 52, overlay.desired_height(52).max(10));
    let mut buf = Buffer::empty(area);
    overlay.render(area, &mut buf);
    let rendered = text(&buf, area);
    let compact = rendered.replace(' ', "");
    assert!(compact.contains("请选择工作方式"), "{rendered}");
    assert!(compact.contains("安全"));
    assert!(rendered.contains("enter to submit"));
}
