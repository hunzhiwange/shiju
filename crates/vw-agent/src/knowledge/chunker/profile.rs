//! 文档分块前的结构特征统计。
//!
//! 负责识别 Markdown 标题与代码围栏，为后续分块策略提供稳定的文档画像。

use super::patterns::{is_fence_marker, parse_markdown_heading};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DocProfile {
    pub total_chars: usize,
    pub total_lines: usize,
    pub markdown_heading_counts: [usize; 6],
    pub markdown_heading_total: usize,
}

impl DocProfile {
    pub(super) fn from_text(text: &str) -> Self {
        let mut profile = Self {
            total_chars: text.chars().count(),
            total_lines: if text.is_empty() { 0 } else { text.split('\n').count() },
            markdown_heading_counts: [0; 6],
            markdown_heading_total: 0,
        };
        let mut in_fence = false;
        for line in text.split('\n') {
            if is_fence_marker(line) {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            if let Some(heading) = parse_markdown_heading(line) {
                if (1..=6).contains(&heading.level) {
                    profile.markdown_heading_counts[heading.level - 1] += 1;
                    profile.markdown_heading_total += 1;
                }
            }
        }
        profile
    }

    pub(super) fn dominant_heading_level(&self) -> Option<usize> {
        if self.markdown_heading_total == 0 {
            return None;
        }
        for (index, count) in self.markdown_heading_counts.iter().enumerate() {
            if *count >= 3 {
                return Some(index + 1);
            }
        }
        self.markdown_heading_counts
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, count)| (*count > 0).then_some(index + 1))
    }

    pub(super) fn should_use_heading_split(&self) -> bool {
        if self.total_lines == 0 || self.markdown_heading_total < 3 {
            return false;
        }
        let heading_density = self.markdown_heading_total as f64 / self.total_lines as f64;
        heading_density > 0.005 && self.dominant_heading_level().is_some()
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod profile_tests;
