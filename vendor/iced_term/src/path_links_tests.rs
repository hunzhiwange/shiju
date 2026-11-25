use super::path_in_line;
use crate::actions::Action;
use crate::backend::{Backend, Command, LinkAction};
use alacritty_terminal::index::{Column, Line, Point};
use iced_core::Size;
use tokio::sync::mpsc;

#[test]
fn recognizes_packaging_and_compiler_paths() {
    for (line, hit, expected) in [
        (
            "--prepared:/tmp/ShiJu Computer Use.app",
            "Computer",
            "/tmp/ShiJu Computer Use.app",
        ),
        (
            "target-dir/osx/ShiJu Computer Use.app: valid on disk",
            "Use.app",
            "target-dir/osx/ShiJu Computer Use.app",
        ),
        (
            "Done! App bundle is ready at: target-dir/osx/ShiJu.app",
            "bundle is",
            "",
        ),
        (
            "Done! App bundle is ready at: target-dir/osx/ShiJu.app",
            "ShiJu",
            "target-dir/osx/ShiJu.app",
        ),
        (
            "You can run it with: open \"target-dir/中文 目录/ShiJu.app\"",
            "目录",
            "target-dir/中文 目录/ShiJu.app",
        ),
        (
            "  --> crates/ui/src/main.rs:155:15",
            "main.rs",
            "crates/ui/src/main.rs",
        ),
        (
            "See ~/Documents/notes.md, next",
            "notes",
            "~/Documents/notes.md",
        ),
        ("choose a / b", "/", ""),
    ] {
        let range = path_in_line(line, line.find(hit).unwrap());
        assert_eq!(range.map_or("", |range| &line[range]), expected, "{line}");
    }
}

#[test]
fn resolves_wrapped_grid_paths_and_rechecks_the_click_position() {
    let (sender, _receiver) = mpsc::channel(16);
    let mut backend = Backend::attach(sender, None, Size::new(1.0, 1.0));
    backend.handle(Command::Resize(Some(Size::new(18.0, 8.0)), None));
    backend.feed_output(
        b"--validated:/tmp/ShiJu Computer Use.app\r\nplain",
        false,
    );

    backend.sync();
    let path_cell = Point::new(Line(1), Column(8));
    backend.handle(Command::ProcessLink(LinkAction::Hover, path_cell));
    let range = backend
        .renderable_content()
        .hovered_hyperlink
        .as_ref()
        .unwrap();
    assert_eq!(*range.start(), Point::new(Line(0), Column(12)));
    assert_eq!(range.end().line, Line(2));
    assert_eq!(
        backend.handle(Command::ProcessLink(
            LinkAction::Open,
            Point::new(Line(3), Column(0))
        )),
        Action::Ignore,
    );
    assert_eq!(
        backend.handle(Command::ProcessLink(LinkAction::Open, path_cell)),
        Action::OpenPath("/tmp/ShiJu Computer Use.app".to_owned()),
    );
}
