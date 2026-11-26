//! 新会话场景及其可填入草稿的推荐。

use std::fmt;

use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_session::message::chat::input::popovers::ComposerScene;
use desktop_ui_controls::system_settings_common::settings_pick_list_menu_style;
use iced::widget::{button, container, pick_list, row, svg, text};
use iced::{Alignment, Border, Element, Pixels, Theme};

use super::quick_message::Message;
use super::surface_styles::{
    BOTTOM_BAR_ICON_BUTTON_SIZE, selector_label_font, selector_pill_button_style,
};

mod categories;

pub(super) struct Prompt {
    pub title: &'static str,
    pub draft: &'static str,
}

fn scene_prompts(scene: ComposerScene) -> &'static [Prompt] {
    match scene {
        ComposerScene::Office => &[
            Prompt {
                title: "整理会议纪要",
                draft: "根据我提供的会议记录，整理关键决策、待办事项、负责人和截止日期。把待确认的内容单列出来。",
            },
            Prompt {
                title: "起草工作周报",
                draft: "根据我提供的工作记录，起草一份简洁的周报，分为已完成、遇到的问题和下周计划。",
            },
            Prompt {
                title: "对比备选方案",
                draft: "根据我提供的需求和备选方案，比较成本、耗时和风险，明确推荐结论及需要补充的信息。",
            },
        ],
        ComposerScene::Development => &[
            Prompt {
                title: "梳理项目结构",
                draft: "梳理当前项目的主要模块和入口，说明它们如何协作，并指出新成员应该先读哪些文件。",
            },
            Prompt {
                title: "制定实现计划",
                draft: "根据我提供的需求和当前代码，拟定实现步骤，标出受影响的模块、边界情况和验证方式。",
            },
            Prompt {
                title: "检查回归风险",
                draft: "审查当前代码变更，找出可能的回归和遗漏的边界情况，给出对应文件及建议的验证方式。",
            },
        ],
        ComposerScene::Design => &[
            Prompt {
                title: "走查界面细节",
                draft: "检查我提供的界面或截图，指出信息层级、对齐、间距和交互反馈的问题，按优先级给出改进建议。",
            },
            Prompt {
                title: "规划页面布局",
                draft: "根据我提供的页面目标和内容，规划首屏布局、主要操作和窄屏表现，并说明设计取舍。",
            },
            Prompt {
                title: "拟定视觉方向",
                draft: "根据我提供的品牌和受众，拟定视觉方向，说明配色、字体、图像风格和可以复用的组件。",
            },
        ],
    }
}

pub fn category_available(scene: ComposerScene, category: Option<usize>) -> bool {
    category.is_none_or(|index| index < categories::categories(scene).len())
}

pub(super) fn prompts(scene: ComposerScene, category: Option<usize>) -> &'static [Prompt] {
    match category {
        None => scene_prompts(scene),
        Some(index) => categories::categories(scene).get(index).map_or(&[], |group| group.prompts),
    }
}

pub fn prompt_draft(
    i18n: I18n,
    scene: ComposerScene,
    category: Option<usize>,
    index: usize,
) -> Option<&'static str> {
    prompts(scene, category).get(index).map(|prompt| i18n.__(prompt.draft))
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CategoryOption {
    index: Option<usize>,
    label: &'static str,
}

impl fmt::Display for CategoryOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label)
    }
}

pub(super) fn category_selector(
    i18n: I18n,
    scene: ComposerScene,
    category: Option<usize>,
) -> Element<'static, Message> {
    use iced::widget::pick_list::Status;

    let mut options = vec![CategoryOption { index: None, label: i18n.__("通用推荐") }];
    options.extend(
        categories::categories(scene).iter().enumerate().map(|(index, group)| CategoryOption {
            index: Some(index),
            label: i18n.__(group.label),
        }),
    );
    let selected = category
        .and_then(|index| options.iter().find(|option| option.index == Some(index)).copied());
    pick_list(options, selected, move |option| {
        Message::ComposerCategorySelected(scene, option.index)
    })
    .placeholder(i18n.__("任务分类"))
    .text_size(12)
    .text_line_height(text::LineHeight::Absolute(Pixels(BOTTOM_BAR_ICON_BUTTON_SIZE - 16.0)))
    .padding([8, 10])
    .style(|theme, status| {
        let (status, open) = match status {
            Status::Active => (button::Status::Active, false),
            Status::Hovered => (button::Status::Hovered, false),
            Status::Opened { .. } => (button::Status::Active, true),
        };
        let style = super::quick_styles::quick_message_group_button_style(theme, status, open);
        pick_list::Style {
            background: style
                .background
                .unwrap_or_else(|| theme.palette().text.scale_alpha(0.06).into()),
            border: style.border,
            text_color: style.text_color,
            placeholder_color: style.text_color.scale_alpha(0.85),
            handle_color: style.text_color.scale_alpha(0.65),
        }
    })
    .menu_style(settings_pick_list_menu_style)
    .into()
}

pub fn scene_selector<Message: Clone + 'static>(
    i18n: I18n,
    scene: ComposerScene,
    compact: bool,
    select: fn(ComposerScene) -> Message,
) -> Element<'static, Message> {
    let mut segments = row![].spacing(2).align_y(Alignment::Center);
    for (option, label, icon) in [
        (ComposerScene::Office, "办公", Icon::FileText),
        (ComposerScene::Development, "开发", Icon::Code),
        (ComposerScene::Design, "设计", Icon::Palette),
    ] {
        let selected = option == scene;
        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if !compact {
            content =
                content.push(super::icons::icon_svg(icon, 15.0).style(move |theme: &Theme, _| {
                    svg::Style {
                        color: Some(theme.palette().text.scale_alpha(if selected {
                            0.95
                        } else {
                            0.70
                        })),
                    }
                }));
        }
        let label = text(i18n.__(label))
            .size(if compact { 12 } else { 13 })
            .font(selector_label_font())
            .wrapping(text::Wrapping::None);
        let segment = button(container(content.push(label)).center_y(BOTTOM_BAR_ICON_BUTTON_SIZE))
            .padding([0, if compact { 8 } else { 12 }])
            .style(move |theme, status| {
                let mut style = selector_pill_button_style(theme, status, selected);
                if selected {
                    let fill = if matches!(status, button::Status::Pressed) { 0.21 } else { 0.15 };
                    style.background = Some(theme.palette().text.scale_alpha(fill).into());
                } else if status == button::Status::Active {
                    style.text_color = theme.palette().text.scale_alpha(0.70);
                }
                style
            })
            .on_press(select(option));
        segments = segments.push(segment);
    }

    container(segments)
        .padding(3)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().text.scale_alpha(0.06).into()),
            border: Border::default().rounded(999.0),
            ..Default::default()
        })
        .into()
}
