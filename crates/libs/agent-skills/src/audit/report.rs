/// 技能审计报告
///
/// 包含技能目录或文件的审计结果，记录扫描的文件数量和发现的所有问题。
///
/// # 字段说明
///
/// - `files_scanned`：已扫描的文件总数
/// - `findings`：发现的安全问题列表，每项都是人类可读的描述字符串
///
/// # 示例
///
/// ```ignore
/// let report = audit_skill_directory(Path::new("./my-skill"))?;
///
/// if report.is_clean() {
///     println!("审计通过，扫描了 {} 个文件", report.files_scanned);
/// } else {
///     println!("发现 {} 个问题:", report.findings.len());
///     for finding in &report.findings {
///         println!("  - {}", finding);
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SkillAuditReport {
    /// 已扫描的文件数量
    pub files_scanned: usize,
    /// 发现的安全问题列表
    pub findings: Vec<String>,
}

impl SkillAuditReport {
    /// 检查审计报告是否干净
    ///
    /// 如果没有发现任何安全问题，返回 `true`。
    ///
    /// # 返回值
    ///
    /// - `true`：没有发现问题，技能可以安全使用
    /// - `false`：存在至少一个安全问题
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if report.is_clean() {
    ///     install_skill(skill_dir)?;
    /// }
    /// ```
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// 生成问题摘要字符串
    ///
    /// 将所有发现的问题用分号连接成一个字符串，便于日志记录或错误消息。
    ///
    /// # 返回值
    ///
    /// 返回所有问题的拼接字符串。如果没有问题，返回空字符串。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if !report.is_clean() {
    ///     log::warn!("技能审计失败: {}", report.summary());
    /// }
    /// ```
    pub fn summary(&self) -> String {
        self.findings.join("; ")
    }
}
#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
