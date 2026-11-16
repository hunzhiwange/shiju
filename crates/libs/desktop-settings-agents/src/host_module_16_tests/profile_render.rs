use super::profile_update::{ProfileFixture, dispatch, runtime_creation_draft};
use crate::agents_update_message::AgentsMessage;
use crate::message::Event;
use iced::Element;
pub(super) fn overlay<'a>(app: &'a ProfileFixture, base: Element<'a, Event>) -> Element<'a, Event> {
    let card = crate::profile_modal::page_card(crate::profile_modal::PageInput {
        state: &app.agents_settings,
        locale: app.desktop_locale,
        theme: app.app_theme,
        help: super::selectable_help,
        acp_agents: &app.acp_agents,
        acp_loading: app.acp_settings.loading,
        acp_error: app.acp_settings.save_error.as_ref(),
        inspect: app.mcp_settings.inspect.as_ref(),
        inspect_loading: app.mcp_settings.inspect_loading,
        inspect_error: app.mcp_settings.inspect_error.as_ref(),
        model_source_available: app.agents_settings.profile_modal.as_ref().is_none_or(|draft| {
            crate::defaults::supports_fixed_model(
                &draft.runtime,
                app.capabilities.iter().map(|(agent, caps)| (agent.as_deref(), caps.as_slice())),
            )
        }),
        workspace_metadata: None,
        project_options: crate::agents_view::agent_project_options(
            app.project_path.as_deref(),
            &app.recent_projects,
        ),
    });
    desktop_ui_controls::system_settings_common::settings_optional_modal_overlay(
        base,
        card.map(|card| (Event::Agents(AgentsMessage::ProfileCancel), card)),
    )
}
#[tokio::test(flavor = "multi_thread")]
async fn agent_config_cleanup_five_tabs_render_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name, tab) in [
        (Theme::Light, "light", crate::state::AGENT_DETAIL_BASIC_TAB),
        (Theme::Dark, "dark", crate::state::AGENT_DETAIL_BASIC_TAB),
        (Theme::Light, "light", crate::state::AGENT_DETAIL_IDENTITY_TAB),
        (Theme::Dark, "dark", crate::state::AGENT_DETAIL_IDENTITY_TAB),
        (Theme::Light, "light", crate::state::AGENT_DETAIL_TOOLS_TAB),
        (Theme::Dark, "dark", crate::state::AGENT_DETAIL_TOOLS_TAB),
        (Theme::Light, "light", crate::state::AGENT_DETAIL_SKILLS_TAB),
        (Theme::Dark, "dark", crate::state::AGENT_DETAIL_SKILLS_TAB),
        (Theme::Light, "light", crate::state::AGENT_DETAIL_MCP_TAB),
        (Theme::Dark, "dark", crate::state::AGENT_DETAIL_MCP_TAB),
    ] {
        for (width, height) in [(1100, 900), (760, 520)] {
            let mut app = ProfileFixture::default();
            app.agents_settings.config_ready = true;
            dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
            dispatch(
                &mut app,
                AgentsMessage::ProfileNameChanged("中文智能体资料与 English".repeat(8)),
            );
            dispatch(
                &mut app,
                AgentsMessage::ProfileDescriptionAction(iced::widget::text_editor::Action::Edit(
                    iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                        "中文长描述，用于验证字形整形和暗黑主题。\n多行描述。\n".repeat(10),
                    )),
                )),
            );
            dispatch(
                &mut app,
                AgentsMessage::SystemPromptAction(
                    "main".into(),
                    iced::widget::text_editor::Action::Edit(
                        iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
                            "规则：审查代码并解释原因。\n".repeat(20),
                        )),
                    ),
                ),
            );
            app.agents_settings.active_detail_tab = tab.to_string();
            let mut element = overlay(
                &app,
                iced::widget::container(iced::widget::text("背景"))
                    .width(iced::Length::Fill)
                    .height(iced::Length::Fill)
                    .into(),
            );
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let bounds = Size::new(width as f32, height as f32);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            #[derive(Default)]
            struct Labels(Vec<String>, usize, Vec<(String, Rectangle)>);
            impl iced::advanced::widget::Operation for Labels {
                fn traverse(
                    &mut self,
                    operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
                ) {
                    operate(self);
                }
                fn focusable(
                    &mut self,
                    _id: Option<&iced::advanced::widget::Id>,
                    _bounds: Rectangle,
                    _state: &mut dyn iced::advanced::widget::operation::Focusable,
                ) {
                    self.1 += 1;
                }
                fn text(
                    &mut self,
                    _id: Option<&iced::advanced::widget::Id>,
                    bounds: Rectangle,
                    text: &str,
                ) {
                    self.0.push(text.to_owned());
                    self.2.push((text.to_owned(), bounds));
                }
            }
            let mut labels = Labels::default();
            element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
            assert!(!labels.0.iter().any(|text| text == "资料与默认参数" || text == "规则"));
            for label in ["基础参数", "身份", "工具", "技能", "MCP", "取消", "保存配置"]
            {
                assert!(labels.0.iter().any(|text| text == label), "missing {label} on {tab}");
            }
            for label in ["智能体配置", "取消", "保存配置"] {
                let bounds = labels.2.iter().find(|(text, _)| text == label).unwrap().1;
                assert!(
                    bounds.y >= 0.0 && bounds.y + bounds.height <= height as f32,
                    "fixed header/footer clipped: {label} {bounds:?}"
                );
            }
            let count = |label: &str| labels.0.iter().filter(|text| text.as_str() == label).count();
            if tab == crate::state::AGENT_DETAIL_BASIC_TAB {
                assert_eq!(count("名称"), 1);
                assert_eq!(count("描述"), 1);
                assert_eq!(count("模型"), 1);
                assert_eq!(count("系统提示词"), 0);
            } else if tab == crate::state::AGENT_DETAIL_IDENTITY_TAB {
                assert_eq!(count("系统提示词"), 2); // tab and field label
                assert_eq!(labels.1, 1); // a single editable system prompt
                assert_eq!(count("描述"), 0);
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(width, height), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
            image::save_buffer(
                format!("/tmp/shiju-task7-{name}-{tab}-{width}x{height}.png"),
                &pixels,
                width,
                height,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_skill_directory_renders_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = ProfileFixture::default();
        app.agents_settings.config_ready = true;
        app.project_path = Some("/应用项目".into());
        app.recent_projects = vec!["/技能目录甲".into(), "/技能目录乙".into()];
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
        dispatch(&mut app, AgentsMessage::SkillDirectorySelected(Some("/技能目录乙".into())));
        app.agents_settings.active_detail_tab = crate::state::AGENT_DETAIL_SKILLS_TAB.into();
        let mut element = overlay(
            &app,
            iced::widget::container(iced::widget::text("背景"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
        );
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(1100.0, 900.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        assert_eq!(node.bounds().size(), bounds);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-task2-skill-directory-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_runtime_creation_controls_render_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for state in ["new", "loading", "failed", "saved"] {
            let mut app = runtime_creation_draft();
            if state == "saved" {
                dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
                dispatch(&mut app, AgentsMessage::ProfileSave);
                dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
                dispatch(&mut app, AgentsMessage::ProfileOpen("runtime-custom".into()));
            } else if state != "new" {
                app.acp_agents.clear();
                app.acp_settings.loading = state == "loading";
                if state == "failed" {
                    app.acp_settings.save_error = Some("目录加载失败，请重试".into());
                }
            }
            let draft = app.agents_settings.profile_modal.as_ref().unwrap();
            let mut element = crate::basic_profile::view(
                crate::basic_profile::Input {
                    locale: app.desktop_locale,
                    entries: &app.agents_settings.entries,
                    acp_agents: &app.acp_agents,
                    saving: app.agents_settings.profile_saving,
                    acp_loading: app.acp_settings.loading,
                    acp_error: app.acp_settings.save_error.as_ref(),
                },
                draft,
            );
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let bounds = Size::new(920.0, 760.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            #[derive(Default)]
            struct Labels(Vec<String>);
            impl iced::advanced::widget::Operation for Labels {
                fn traverse(
                    &mut self,
                    operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
                ) {
                    operate(self);
                }
                fn text(
                    &mut self,
                    _id: Option<&iced::advanced::widget::Id>,
                    _bounds: Rectangle,
                    value: &str,
                ) {
                    self.0.push(value.into());
                }
            }
            let mut labels = Labels::default();
            element.as_widget_mut().operate(&mut tree, Layout::new(&node), &renderer, &mut labels);
            assert_eq!(labels.0.iter().any(|s| s.contains("首次保存成功后")), state != "saved");
            if state == "saved" {
                assert!(labels.0.iter().any(|s| s == "固定运行引擎：codex"));
                assert!(!labels.0.iter().any(|s| s == "刷新引擎目录"));
            } else if state == "loading" {
                assert!(labels.0.iter().any(|s| s == "正在加载 ACP 引擎目录…"));
            } else if state == "failed" {
                assert!(labels.0.iter().any(|s| s == "目录加载失败，请重试"));
            }
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(920, 760), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
            image::save_buffer(
                format!("/tmp/shiju-runtime-creation-{name}-{state}.png"),
                &pixels,
                920,
                760,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_runtime_visibility_actual_modal_renders_native_and_acp_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for backend in ["native", "codex", "claude"] {
        for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            let mut app = runtime_creation_draft();
            dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
            for tab in ["basic", "identity", "skills", "mcp", "tools"] {
                // Exercise the view's defensive fallback as well as reducer validation.
                app.agents_settings.active_detail_tab = tab.into();
                let mut element = overlay(
                    &app,
                    iced::widget::container(iced::widget::text("背景"))
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                        .into(),
                );
                let mut renderer = <iced::Renderer as Headless>::new(
                    Font::DEFAULT,
                    Pixels(16.0),
                    Some("tiny-skia"),
                )
                .await
                .unwrap();
                let bounds = Size::new(1100.0, 900.0);
                let mut tree = Tree::new(element.as_widget());
                let node = element.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, bounds),
                );
                #[derive(Default)]
                struct Labels(Vec<String>);
                impl iced::advanced::widget::Operation for Labels {
                    fn traverse(
                        &mut self,
                        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
                    ) {
                        operate(self);
                    }
                    fn text(
                        &mut self,
                        _: Option<&iced::advanced::widget::Id>,
                        _: Rectangle,
                        value: &str,
                    ) {
                        self.0.push(value.into());
                    }
                }
                let mut labels = Labels::default();
                element.as_widget_mut().operate(
                    &mut tree,
                    Layout::new(&node),
                    &renderer,
                    &mut labels,
                );
                let has = |label: &str| labels.0.iter().any(|s| s == label);
                for label in ["基础参数", "身份", "技能", "MCP", "保存配置"] {
                    assert!(has(label), "{backend}/{tab}: {label}");
                }
                assert_eq!(has("工具"), backend == "native");
                if tab == "skills" {
                    assert!(has("技能目录"));
                }
                if tab == "mcp" {
                    assert!(has("默认 MCP"));
                }
                if tab == "identity" {
                    assert!(has("系统提示词"));
                }
                if tab == "basic" || (tab == "tools" && backend != "native") {
                    for label in ["名称", "描述", "模型", "权限", "作为子智能体"] {
                        assert!(has(label), "{backend}: {label}");
                    }
                    for label in [
                        "API 密钥",
                        "温度",
                        "最大深度",
                        "子智能体数量",
                        "最大迭代次数",
                        "智能体模式",
                        "上下文压缩",
                        "历史消息上限",
                        "并行工具",
                    ] {
                        assert_eq!(has(label), backend == "native", "{backend}: {label}");
                    }
                }
                element.as_widget().draw(
                    &tree,
                    &mut renderer,
                    &theme,
                    &renderer::Style { text_color: theme.palette().text },
                    Layout::new(&node),
                    mouse::Cursor::Unavailable,
                    &Rectangle::with_size(bounds),
                );
                let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
                assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
                if tab == "basic" {
                    image::save_buffer(
                        format!("/tmp/shiju-runtime-visibility-{backend}-{name}.png"),
                        &pixels,
                        1100,
                        900,
                        image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
            }
        }
    }
}
