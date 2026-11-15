use super::*;
use insta::assert_snapshot;

fn pager() -> TranscriptPager {
    TranscriptPager::new(
        (0..24).map(|index| Line::from(format!("transcript line {index:02}"))).collect(),
    )
}

fn buffer_text(buffer: &Buffer, area: Rect) -> String {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
// 初始末尾、跳到顶部和手动下移应分别显示对应位置标签。
fn transcript_pager_tracks_top_middle_and_end() {
    let area = Rect::new(0, 0, 40, 10);
    let mut pager = pager();
    let mut buffer = Buffer::empty(area);
    pager.render(area, &mut buffer);
    assert!(buffer_text(&buffer, area).contains(" end "));

    pager.scroll_to_top();
    pager.render(area, &mut buffer);
    assert!(buffer_text(&buffer, area).contains(" top "));

    pager.scroll_lines(5);
    pager.render(area, &mut buffer);
    assert!(buffer_text(&buffer, area).contains(" middle "));
}

#[test]
// 手动偏移在缩放后保持，而末尾跟随应按新视口重新钉住底部。
fn transcript_pager_resize_preserves_manual_position_and_end_pin() {
    let mut pager = pager();
    let mut wide = Buffer::empty(Rect::new(0, 0, 60, 12));
    pager.render(*wide.area(), &mut wide);
    pager.scroll_to_top();
    pager.scroll_lines(4);

    let mut narrow = Buffer::empty(Rect::new(0, 0, 28, 7));
    pager.render(*narrow.area(), &mut narrow);
    assert_eq!(pager.scroll_offset, 4);

    pager.scroll_to_end();
    let mut tall = Buffer::empty(Rect::new(0, 0, 60, 18));
    pager.render(*tall.area(), &mut tall);
    assert_eq!(pager.scroll_offset, max_scroll_offset(24, 15));
}

#[test]
// 快照覆盖窄、标准和宽三种尺寸下的边框、正文、滚动条与页脚。
fn transcript_pager_three_size_snapshots() {
    let mut output = String::new();
    for (name, area) in [
        ("narrow", Rect::new(0, 0, 28, 7)),
        ("standard", Rect::new(0, 0, 40, 10)),
        ("wide", Rect::new(0, 0, 60, 12)),
    ] {
        let mut pager = pager();
        pager.scroll_to_top();
        pager.scroll_lines(3);
        let mut buffer = Buffer::empty(area);
        pager.render(area, &mut buffer);
        output.push_str(&format!("--- {name} ---\n{}\n", buffer_text(&buffer, area)));
    }
    assert_snapshot!("v3_transcript_pager_three_sizes", output);
}
