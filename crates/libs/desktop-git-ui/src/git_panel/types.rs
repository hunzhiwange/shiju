#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConventionalCommitType {
    /// 新功能（feat）
    Feat,
    /// Bug 修复（fix）
    Fix,
    /// 文档变更（docs）
    Docs,
    /// 代码格式调整（style）
    Style,
    /// 代码重构（refactor）
    Refactor,
    /// 性能优化（perf）
    Perf,
    /// 测试相关（test）
    Test,
    /// 构建系统（build）
    Build,
    /// CI/CD 配置（ci）
    Ci,
    /// 其他修改（chore）
    Chore,
    /// 回退提交（revert）
    Revert,
    /// 初始化（init）
    Init,
    /// 配置变更（config）
    Config,
    /// 发布（release）
    Release,
    /// 部署（deploy）
    Deploy,
    /// 合并（merge）
    Merge,
    /// 进行中（wip）
    Wip,
    /// 拼写错误（typo）
    Typo,
    /// 国际化（locale）
    Locale,
}

impl ConventionalCommitType {
    /// 获取所有提交类型的数组
    ///
    /// # 返回值
    ///
    /// 返回包含所有 `ConventionalCommitType` 变体的数组
    pub const fn all() -> [ConventionalCommitType; 19] {
        [
            ConventionalCommitType::Feat,
            ConventionalCommitType::Fix,
            ConventionalCommitType::Docs,
            ConventionalCommitType::Style,
            ConventionalCommitType::Refactor,
            ConventionalCommitType::Perf,
            ConventionalCommitType::Test,
            ConventionalCommitType::Build,
            ConventionalCommitType::Ci,
            ConventionalCommitType::Chore,
            ConventionalCommitType::Revert,
            ConventionalCommitType::Init,
            ConventionalCommitType::Config,
            ConventionalCommitType::Release,
            ConventionalCommitType::Deploy,
            ConventionalCommitType::Merge,
            ConventionalCommitType::Wip,
            ConventionalCommitType::Typo,
            ConventionalCommitType::Locale,
        ]
    }

    /// 将提交类型转换为字符串
    ///
    /// # 返回值
    ///
    /// 返回提交类型对应的小写字符串标识
    pub const fn as_str(self) -> &'static str {
        match self {
            ConventionalCommitType::Feat => "feat",
            ConventionalCommitType::Fix => "fix",
            ConventionalCommitType::Docs => "docs",
            ConventionalCommitType::Style => "style",
            ConventionalCommitType::Refactor => "refactor",
            ConventionalCommitType::Perf => "perf",
            ConventionalCommitType::Test => "test",
            ConventionalCommitType::Build => "build",
            ConventionalCommitType::Ci => "ci",
            ConventionalCommitType::Chore => "chore",
            ConventionalCommitType::Revert => "revert",
            ConventionalCommitType::Init => "init",
            ConventionalCommitType::Config => "config",
            ConventionalCommitType::Release => "release",
            ConventionalCommitType::Deploy => "deploy",
            ConventionalCommitType::Merge => "merge",
            ConventionalCommitType::Wip => "wip",
            ConventionalCommitType::Typo => "typo",
            ConventionalCommitType::Locale => "locale",
        }
    }

    /// 从前缀字符串解析提交类型
    ///
    /// # 参数
    ///
    /// - `s`：前缀字符串（如 "feat"、"fix" 等）
    ///
    /// # 返回值
    ///
    /// 如果匹配成功，返回对应的 `ConventionalCommitType`；
    /// 否则返回 `None`
    #[allow(dead_code)]
    pub fn from_prefix(s: &str) -> Option<ConventionalCommitType> {
        match s {
            "feat" => Some(ConventionalCommitType::Feat),
            "fix" => Some(ConventionalCommitType::Fix),
            "docs" => Some(ConventionalCommitType::Docs),
            "style" => Some(ConventionalCommitType::Style),
            "refactor" => Some(ConventionalCommitType::Refactor),
            "perf" => Some(ConventionalCommitType::Perf),
            "test" => Some(ConventionalCommitType::Test),
            "build" => Some(ConventionalCommitType::Build),
            "ci" => Some(ConventionalCommitType::Ci),
            "chore" => Some(ConventionalCommitType::Chore),
            "revert" => Some(ConventionalCommitType::Revert),
            "init" => Some(ConventionalCommitType::Init),
            "config" => Some(ConventionalCommitType::Config),
            "release" => Some(ConventionalCommitType::Release),
            "deploy" => Some(ConventionalCommitType::Deploy),
            "merge" => Some(ConventionalCommitType::Merge),
            "wip" => Some(ConventionalCommitType::Wip),
            "typo" => Some(ConventionalCommitType::Typo),
            "locale" => Some(ConventionalCommitType::Locale),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConventionalCommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Git 面板可切换的工作树选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeOption {
    pub directory: String,
    pub label: String,
    pub branch: Option<String>,
}

impl std::fmt::Display for GitWorktreeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
