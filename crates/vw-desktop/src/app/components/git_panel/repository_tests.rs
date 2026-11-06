//! 使用实际字体整形和绘制路径验证中文历史与明暗主题。
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repository_history_renders_chinese_in_both_themes() {
    use super::repository_ops::*;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Font, Pixels, Rectangle, Size, Theme};
    use vw_gateway_client::vw_api_types::git::{
        GitHistoryCommitDto as HistoryCommit, GitHistoryFileDto as HistoryFile,
    };
    // 客户端不必存在这个目录：渲染和交互只依赖网关返回的数据。
    let remote_path = "/gateway-only/中文项目";
    for (theme, name, width) in [
        (Theme::Light, "light", 520),
        (Theme::Dark, "dark", 520),
        (Theme::Light, "light", 760),
        (Theme::Dark, "dark", 760),
        (Theme::Light, "working-light", 760),
        (Theme::Dark, "working-dark", 760),
        (Theme::Light, "preview-light", 760),
        (Theme::Dark, "preview-dark", 760),
    ] {
        let mut app = crate::app::App::new().0;
        let path = remote_path.to_owned();
        app.project_path = Some(path.clone());
        app.app_theme = theme.clone();
        app.show_git_diff_highlight = true;
        app.merge_view = width == 520;
        app.git_repository.history = !name.starts_with("working-");
        app.git_repository.repo_path = Some(path);
        app.git_repository.snapshot = RepositorySnapshot {
            branch: Some("feature/中文历史".into()),
            remote: Some("origin".into()),
            remote_branch: Some("feature/中文历史".into()),
            upstream: true,
            ahead_behind: Some((2, 1)),
            commits: vec![HistoryCommit {
                sha: "a".repeat(40),
                summary: "fix(desktop): 支持中文显示、很长的提交说明以及网关历史文件差异查看"
                    .into(),
                author: "测试作者".into(),
                time: 1_700_000_000,
            }],
            has_more: false,
        };
        app.selected_branch = Some("feature/中文历史".into());
        app.git_repository.selected = Some("a".repeat(40));
        app.git_repository.detail = Some(HistoryDetail {
            message: "修复：中文显示与提交历史\n\n支持作者、文件和历史差异。".into(),
            files: vec![
                HistoryFile {
                    path: "crates/中文模块.rs".into(),
                    status: vw_gateway_client::vw_api_types::git::GitFileStatus::Modified,
                    old_content: Some("fn main() {\n    // 旧内容\n}\n".into()),
                    new_content: Some("fn main() {\n    // 新的中文内容\n}\n".into()),
                    notice: None,
                    insertions: 1,
                    deletions: 1,
                },
                HistoryFile {
                    path: "crates/很长的新增文件路径/新增.rs".into(),
                    status: vw_gateway_client::vw_api_types::git::GitFileStatus::Added,
                    old_content: Some(String::new()),
                    new_content: Some("新文件".into()),
                    notice: None,
                    insertions: 1,
                    deletions: 0,
                },
                HistoryFile {
                    path: "删除文件.txt".into(),
                    status: vw_gateway_client::vw_api_types::git::GitFileStatus::Deleted,
                    old_content: Some("已删除".into()),
                    new_content: Some(String::new()),
                    notice: None,
                    insertions: 0,
                    deletions: 1,
                },
            ],
        });
        app.git_repository.selected_file = Some(0);
        app.git_repository.remote_busy = true;
        app.git_repository.remote_action = Some(RemoteAction::Fetch);
        app.git_repository.remote_animation_frame = 8;
        if !app.git_repository.history {
            let file = app.git_repository.detail.as_ref().unwrap().files[0].clone();
            let old = file.old_content.unwrap();
            let new = file.new_content.unwrap();
            app.show_git_diff_summary = false;
            app.git_diff_file_metas = vec![super::DiffFileMeta {
                path: file.path.clone(),
                status: super::utils::FileStatus::Modified,
                old_oid: None,
                old_size: old.len() as u64,
                new_size: new.len() as u64,
                new_exists: true,
                insertions: 1,
                deletions: 1,
            }];
            app.git_diff_contents.insert(file.path.clone(), (old, new));
            app.git_diff_file_metas_repo_path = app.project_path.clone();
            app.set_single_expanded_file(file.path);
        }
        if name.starts_with("preview-") {
            let task = crate::app::message::git::update(
                &mut app,
                crate::app::message::GitMessage::PreviewHistoryFile(0),
            );
            use iced_runtime::futures::futures::StreamExt;
            let mut stream = iced_runtime::task::into_stream(task).unwrap();
            while let Some(action) = stream.next().await {
                if let iced_runtime::Action::Output(crate::app::Message::Preview(message)) = action
                {
                    let _ = crate::app::message::preview::update(&mut app, message);
                }
            }
            assert!(!app.show_git_copy_modal);
            assert!(app.preview_tabs[0].read_only);
        } else if app.git_repository.history {
            app.git_repository.file_menu = Some(1);
        }
        let mut element = if name.starts_with("preview-") {
            crate::app::components::preview_panel::view(&app)
        } else {
            super::view(&app)
        };
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(width as f32, 650.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        if app.git_repository.history && !name.starts_with("preview-") {
            let mut overlay = element
                .as_widget_mut()
                .overlay(
                    &mut tree,
                    Layout::new(&node),
                    &renderer,
                    &Rectangle::with_size(bounds),
                    iced::Vector::ZERO,
                )
                .expect("history file menu must expose its dropdown overlay");
            let menu_node = overlay.as_overlay_mut().layout(&renderer, bounds);
            overlay.as_overlay().draw(
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&menu_node),
                mouse::Cursor::Unavailable,
            );
        }
        let pixels = renderer.screenshot(Size::new(width, 650), 1.0, theme.palette().background);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-git-history-{name}-{width}.png"),
            &pixels,
            width,
            650,
            image::ColorType::Rgba8,
        )
        .unwrap();
        let added = pixels
            .chunks_exact(4)
            .filter(|pixel| {
                pixel[1] > pixel[0].saturating_add(8) && pixel[1] > pixel[2].saturating_add(8)
            })
            .count();
        let deleted = pixels
            .chunks_exact(4)
            .filter(|pixel| {
                pixel[0] > pixel[1].saturating_add(8) && pixel[0] > pixel[2].saturating_add(8)
            })
            .count();
        if name.starts_with("preview-") {
            continue;
        }
        assert!(added > 500, "added rows must remain visible: {name} {width}, pixels={added}");
        assert!(
            deleted > 500,
            "deleted rows must remain visible: {name} {width}, pixels={deleted}"
        );
    }
}

#[test]
fn history_labels_truncate_without_splitting_unicode() {
    assert_eq!(super::repository::ellipsize("short", 10), "short");
    assert_eq!(super::repository::ellipsize("中文提交标题很长", 9), "中文提交…");
}
