use crate::permission::Ruleset;
use crate::snapshot::FileDiff;
use serde::{Deserialize, Serialize};

const PLAN_RUN_SLUG_PREFIX: &str = "plan-run:";

/// 会话摘要中的文件与行数统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub additions: i64,
    pub deletions: i64,
    pub files: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diffs: Option<Vec<FileDiff>>,
}

/// 会话分享后的外部链接信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInfo {
    pub url: String,
}

/// 会话生命周期时间信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInfo {
    pub created: u64,
    pub updated: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacting: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<u64>,
}

/// 会话回滚所需的引用信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertInfo {
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(rename = "partID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// 代理会话的共享层元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub id: String,
    pub slug: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub directory: String,
    #[serde(rename = "parentID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<ShareInfo>,
    pub title: String,
    pub version: String,
    pub time: TimeInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Ruleset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert: Option<RevertInfo>,
}

impl Info {
    /// 将普通 slug 编码为可持久化的 plan-run 会话标记。
    pub fn plan_run_slug(slug: &str) -> String {
        if slug.starts_with(PLAN_RUN_SLUG_PREFIX) {
            slug.to_string()
        } else {
            format!("{PLAN_RUN_SLUG_PREFIX}{slug}")
        }
    }

    /// 判断当前会话是否由 plan-run 创建。
    pub fn is_plan_run(&self) -> bool {
        self.slug.starts_with(PLAN_RUN_SLUG_PREFIX)
    }
}

#[cfg(test)]
#[path = "info_tests.rs"]
mod info_tests;
