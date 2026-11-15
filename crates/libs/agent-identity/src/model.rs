use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AIEOS v1.1 身份结构。
///
/// 遵循 AIEOS 规范定义 AI 代理的身份、性格和行为特征。
/// 完整规范参见 https://aieos.org。
///
/// # 字段说明
///
/// - `identity`: 核心身份信息，包括姓名、简介、出生地、居住地
/// - `psychology`: 心理特征，包括认知权重、MBTI、OCEAN 五因素模型、道德指南针
/// - `linguistics`: 语言风格，包括文本风格、正式程度、口头禅、禁用词
/// - `motivations`: 动机驱动，包括核心驱动力、短期/长期目标、恐惧
/// - `capabilities`: 能力定义，包括技能和可访问的工具
/// - `physicality`: 外貌描述，用于图像生成的视觉描述符
/// - `history`: 背景历史，包括起源故事、教育经历、职业
/// - `interests`: 兴趣爱好，包括爱好、喜好、生活方式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AieosIdentity {
    /// 核心身份：姓名、简介、出生地、居住地
    #[serde(default)]
    pub identity: Option<IdentitySection>,

    /// 心理特征：认知权重、MBTI、OCEAN、道德指南针
    #[serde(default)]
    pub psychology: Option<PsychologySection>,

    /// 语言风格：文本风格、正式程度、口头禅、禁用词
    #[serde(default)]
    pub linguistics: Option<LinguisticsSection>,

    /// 动机驱动：核心驱动力、目标、恐惧
    #[serde(default)]
    pub motivations: Option<MotivationsSection>,

    /// 能力定义：代理可访问的技能和工具
    #[serde(default)]
    pub capabilities: Option<CapabilitiesSection>,

    /// 外貌描述：用于图像生成的视觉描述符
    #[serde(default)]
    pub physicality: Option<PhysicalitySection>,

    /// 背景历史：起源故事、教育经历、职业
    #[serde(default)]
    pub history: Option<HistorySection>,

    /// 兴趣爱好：爱好、喜好、生活方式
    #[serde(default)]
    pub interests: Option<InterestsSection>,
}

/// 身份信息段。
///
/// 包含代理的核心身份信息，如姓名、个人简介、出生地和居住地。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IdentitySection {
    /// 姓名（名、姓、昵称、全名）
    #[serde(default)]
    pub names: Option<Names>,

    /// 个人简介
    #[serde(default)]
    pub bio: Option<String>,

    /// 出生地/来源地
    #[serde(default)]
    pub origin: Option<String>,

    /// 当前居住地
    #[serde(default)]
    pub residence: Option<String>,
}

/// 姓名结构。
///
/// 支持多种姓名表示方式：名、姓、昵称和全名。
/// 如果未提供全名但提供了名和姓，系统会自动拼接。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Names {
    /// 名（名字）
    #[serde(default)]
    pub first: Option<String>,

    /// 姓（姓氏）
    #[serde(default)]
    pub last: Option<String>,

    /// 昵称/别名
    #[serde(default)]
    pub nickname: Option<String>,

    /// 全名（完整姓名）
    #[serde(default)]
    pub full: Option<String>,
}

/// 心理特征段。
///
/// 定义代理的心理特征，包括认知权重矩阵、MBTI 人格类型、
/// OCEAN 五因素人格模型和道德指南针。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PsychologySection {
    /// 神经矩阵（认知权重），键为特征名，值为权重值
    #[serde(default)]
    pub neural_matrix: Option<HashMap<String, f64>>,

    /// MBTI 人格类型（如 "INTJ"、"ENFP" 等）
    #[serde(default)]
    pub mbti: Option<String>,

    /// OCEAN 五因素人格模型
    #[serde(default)]
    pub ocean: Option<OceanTraits>,

    /// 道德指南针（核心价值观和原则）
    #[serde(default)]
    pub moral_compass: Option<Vec<String>>,
}

/// OCEAN 五因素人格模型。
///
/// 也称为大五人格模型，包含五个核心人格维度：
/// - 开放性（Openness）：好奇心、创造力
/// - 尽责性（Conscientiousness）：组织性、自律性
/// - 外向性（Extraversion）：社交性、活力
/// - 宜人性（Agreeableness）：合作性、同理心
/// - 神经质（Neuroticism）：情绪稳定性
///
/// 每个维度的值通常在 0.0 到 1.0 之间。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OceanTraits {
    /// 开放性：好奇心、创造力程度
    #[serde(default)]
    pub openness: Option<f64>,

    /// 尽责性：组织性、自律性程度
    #[serde(default)]
    pub conscientiousness: Option<f64>,

    /// 外向性：社交性、活力程度
    #[serde(default)]
    pub extraversion: Option<f64>,

    /// 宜人性：合作性、同理心程度
    #[serde(default)]
    pub agreeableness: Option<f64>,

    /// 神经质：情绪不稳定性程度
    #[serde(default)]
    pub neuroticism: Option<f64>,
}

/// 语言风格段。
///
/// 定义代理的语言表达特征，包括文本风格、正式程度、
/// 常用口头禅以及应避免使用的词语。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinguisticsSection {
    /// 文本风格描述（如 "casual"、"professional" 等）
    #[serde(default)]
    pub style: Option<String>,

    /// 正式程度（如 "formal"、"informal" 或数值）
    #[serde(default)]
    pub formality: Option<String>,

    /// 口头禅/常用语列表
    #[serde(default)]
    pub catchphrases: Option<Vec<String>>,

    /// 禁用词/应避免的词语列表
    #[serde(default)]
    pub forbidden_words: Option<Vec<String>>,
}

/// 动机驱动段。
///
/// 定义代理的内在动机，包括核心驱动力、
/// 短期和长期目标，以及恐惧/回避事项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MotivationsSection {
    /// 核心驱动力（代理行为的主要动机）
    #[serde(default)]
    pub core_drive: Option<String>,

    /// 短期目标列表
    #[serde(default)]
    pub short_term_goals: Option<Vec<String>>,

    /// 长期目标列表
    #[serde(default)]
    pub long_term_goals: Option<Vec<String>>,

    /// 恐惧/回避事项列表
    #[serde(default)]
    pub fears: Option<Vec<String>>,
}

/// 能力定义段。
///
/// 定义代理具备的技能和可访问的工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitiesSection {
    /// 技能列表
    #[serde(default)]
    pub skills: Option<Vec<String>>,

    /// 可访问的工具列表
    #[serde(default)]
    pub tools: Option<Vec<String>>,
}

/// 外貌描述段。
///
/// 定义代理的视觉外貌描述，主要用于图像生成场景。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PhysicalitySection {
    /// 外貌描述（综合外观描述）
    #[serde(default)]
    pub appearance: Option<String>,

    /// 头像描述（用于生成头像的详细描述）
    #[serde(default)]
    pub avatar_description: Option<String>,
}

/// 背景历史段。
///
/// 定义代理的背景故事，包括起源故事、教育经历和职业。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistorySection {
    /// 起源故事/背景故事
    #[serde(default)]
    pub origin_story: Option<String>,

    /// 教育经历列表
    #[serde(default)]
    pub education: Option<Vec<String>>,

    /// 职业/工作
    #[serde(default)]
    pub occupation: Option<String>,
}

/// 兴趣爱好段。
///
/// 定义代理的个人兴趣，包括爱好、喜好和生活方式。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InterestsSection {
    /// 爱好列表
    #[serde(default)]
    pub hobbies: Option<Vec<String>>,

    /// 喜好映射（类别 -> 喜好的值）
    #[serde(default)]
    pub favorites: Option<HashMap<String, String>>,

    /// 生活方式描述
    #[serde(default)]
    pub lifestyle: Option<String>,
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
