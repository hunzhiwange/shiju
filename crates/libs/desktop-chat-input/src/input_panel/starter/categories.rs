//! 场景下的任务分类与推荐草稿。

use desktop_session::message::chat::input::popovers::ComposerScene;

use super::Prompt;

pub(super) struct Category {
    pub label: &'static str,
    pub prompts: &'static [Prompt],
}

pub(super) fn categories(scene: ComposerScene) -> &'static [Category] {
    match scene {
        ComposerScene::Office => OFFICE,
        ComposerScene::Development => DEVELOPMENT,
        ComposerScene::Design => DESIGN,
    }
}

static OFFICE: &[Category] = &[
    Category {
        label: "文档创作",
        prompts: &[
            prompt("按我给的提纲起草文档，标出还需要补充的信息。"),
            prompt("把这份笔记整理成一份可协作的项目说明。"),
        ],
    },
    Category {
        label: "财务分析",
        prompts: &[
            prompt("整理我提供的收支表，标出预算偏差和待核对项。"),
            prompt("根据我提供的数字汇总月度现金流，并注明假设。"),
        ],
    },
    Category {
        label: "数据分析",
        prompts: &[
            prompt("梳理表格字段、缺失值和异常项，给出清洗建议。"),
            prompt("比较各组数据的变化，推荐图表并总结主要发现。"),
        ],
    },
    Category {
        label: "演示文稿",
        prompts: &[
            prompt("把这份材料改成汇报提纲，每页保留一个重点。"),
            prompt("为产品演示规划叙事顺序、页面标题和讲稿。"),
        ],
    },
    Category {
        label: "专题研究",
        prompts: &[
            prompt("围绕这个问题整理研究框架、待查证据和来源。"),
            prompt("对比我提供的资料，标出共识、分歧和证据缺口。"),
        ],
    },
    Category {
        label: "方案策划",
        prompts: &[
            prompt("把目标拆成可执行的方案，写清优先级、里程碑和分工。"),
            prompt("比较两种执行方案，找出关键取舍和依赖。"),
        ],
    },
    Category {
        label: "定时任务",
        prompts: &[
            prompt("为每周例行汇总拟定触发时间、输入和失败提醒。"),
            prompt("列出定时任务的执行步骤、重试条件和输出格式。"),
        ],
    },
];

static DEVELOPMENT: &[Category] = &[
    Category {
        label: "日常开发",
        prompts: &[
            prompt("审查当前改动，找出可能的回归和边界问题。"),
            prompt("梳理当前项目的主要模块、入口和依赖关系。"),
        ],
    },
    Category {
        label: "网站开发",
        prompts: &[
            prompt("为网站需求梳理页面路由、组件和数据流。"),
            prompt("设计一个适合手机和桌面的待办清单页面。"),
        ],
    },
    Category {
        label: "小程序",
        prompts: &[
            prompt("规划小程序的首页、列表和详情页，标出必要交互。"),
            prompt("为小程序列出加载、空列表和提交失败的处理。"),
        ],
    },
    Category {
        label: "智能体应用",
        prompts: &[
            prompt("规划一个能整理笔记和待办的智能体，写清输入和输出。"),
            prompt("为资料问答智能体设计检索、引用和无答案时的反馈。"),
        ],
    },
    Category {
        label: "技能开发",
        prompts: &[
            prompt("把这套重复操作整理成技能，列出触发条件和步骤。"),
            prompt("检查技能的触发说明和示例，找出容易误用的地方。"),
        ],
    },
    Category {
        label: "技术文档",
        prompts: &[
            prompt("根据当前项目起草上手文档，说明运行方式和目录结构。"),
            prompt("把现有接口整理成文档，列出参数、示例和错误说明。"),
        ],
    },
];

static DESIGN: &[Category] = &[
    Category {
        label: "视觉海报",
        prompts: &[
            prompt("为活动海报拟定主视觉、文案层级和版式。"),
            prompt("根据受众和投放渠道，给出两种海报构图。"),
        ],
    },
    Category {
        label: "网站设计",
        prompts: &[
            prompt("规划落地页首屏、内容顺序和窄屏布局。"),
            prompt("检查网站截图中的对齐、间距和交互反馈。"),
        ],
    },
    Category {
        label: "视频生成",
        prompts: &[
            prompt("把这个创意拆成短片分镜，注明镜头、时长和旁白。"),
            prompt("为这组参考图写视频生成提示，说明镜头运动和画面节奏。"),
        ],
    },
    Category {
        label: "演示设计",
        prompts: &[
            prompt("统一演示文稿的字体、配色、网格和图表样式。"),
            prompt("重新组织这套演示文稿的视觉层级，让重点更易读。"),
        ],
    },
    Category {
        label: "运营物料",
        prompts: &[
            prompt("把这次活动的内容转成可复用的社媒物料清单。"),
            prompt("规划同一活动在不同渠道的主图和文案布局。"),
        ],
    },
    Category {
        label: "品牌设计",
        prompts: &[
            prompt("根据品牌定位提出配色、字体和图像风格。"),
            prompt("为这套品牌视觉整理标志、颜色和排版使用规范。"),
        ],
    },
];

const fn prompt(draft: &'static str) -> Prompt {
    Prompt { title: draft, draft }
}
