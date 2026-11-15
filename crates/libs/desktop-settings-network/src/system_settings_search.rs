//! 设置条目的文字与拼音匹配；导航目标由宿主提供。
use std::borrow::Cow;
use pinyin::ToPinyin;
/// 一个可导航系统设置目标的搜索元数据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemSettingsSearchEntry<T> {
    pub original_label: &'static str,
    pub original_parent_label: Option<&'static str>,
    pub ancestor_label: Option<&'static str>,
    /// 搜索结果的主标签。
    pub label: &'static str,
    /// 可选的上级设置分区标签。
    pub parent_label: Option<&'static str>,
    /// 补充检索词，不参与界面展示。
    pub keywords: &'static [&'static str],
    /// 命中后使用的强类型导航目标。
    pub target: T,
}

impl<T: Copy> SystemSettingsSearchEntry<T> {
    pub const fn new(
        label: &'static str,
        parent_label: Option<&'static str>,
        keywords: &'static [&'static str],
        target: T,
    ) -> Self {
        Self {
            label,
            original_label: label,
            original_parent_label: parent_label,
            ancestor_label: None,
            parent_label,
            keywords,
            target,
        }
    }

    /// 使用文字、拼音全拼和首字母形式匹配查询。空查询匹配全部条目。
    pub fn matches(&self, query: &str) -> bool {
        let forms = literal_search_forms(query);
        forms.is_empty() || self.match_rank(&forms).is_some()
    }

    pub fn parent_path(&self) -> Option<Cow<'static, str>> {
        let parent = self.parent_label?;
        Some(match self.ancestor_label {
            Some(ancestor) => Cow::Owned(format!("{parent} > {ancestor}")),
            None => Cow::Borrowed(parent),
        })
    }

    pub fn match_rank(&self, query_forms: &[String]) -> Option<u8> {
        let contains_query =
            |candidate: &str| query_forms.iter().any(|query| candidate.contains(query));
        let label_rank = [self.label, self.original_label]
            .into_iter()
            .filter_map(|label| {
                let forms = search_forms(label);
                if forms.iter().any(|candidate| query_forms.contains(candidate)) {
                    Some(0)
                } else {
                    forms.iter().any(|candidate| contains_query(candidate)).then_some(1)
                }
            })
            .min();
        let matches = |text| search_forms(text).iter().any(|form| contains_query(form));
        if label_rank.is_some() {
            label_rank
        } else if self.keywords.iter().copied().any(matches) {
            Some(2)
        } else if self.ancestor_label.is_none()
            && self.parent_label.into_iter().chain(self.original_parent_label).any(matches)
        {
            Some(3)
        } else {
            None
        }
    }
}

pub fn literal_search_forms(text: &str) -> Vec<String> {
    let literal = normalized_literal(text);
    if literal.is_empty() {
        return Vec::new();
    }

    let compact = literal.replace(' ', "");
    if compact == literal { vec![literal] } else { vec![literal, compact] }
}

fn search_forms(text: &str) -> Vec<String> {
    let mut forms = literal_search_forms(text);
    let mut full_pinyin = String::new();
    let mut spaced_pinyin = String::new();
    let mut initials = String::new();

    for character in text.to_lowercase().chars() {
        if let Some(pinyin) = character.to_pinyin() {
            let plain = pinyin.plain();
            full_pinyin.push_str(plain);
            push_token(&mut spaced_pinyin, plain);
            if let Some(initial) = plain.chars().next() {
                initials.push(initial);
            }
        } else if character.is_alphanumeric() {
            full_pinyin.push(character);
            spaced_pinyin.push(character);
            initials.push(character);
        } else if !spaced_pinyin.ends_with(' ') && !spaced_pinyin.is_empty() {
            spaced_pinyin.push(' ');
        }
    }

    push_unique(&mut forms, full_pinyin);
    push_unique(&mut forms, spaced_pinyin.trim().to_owned());
    push_unique(&mut forms, initials);
    forms
}

fn normalized_literal(text: &str) -> String {
    let mut normalized = String::new();
    let mut pending_space = false;

    for character in text.to_lowercase().chars() {
        if character.is_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(character);
            pending_space = false;
        } else {
            pending_space = true;
        }
    }

    normalized
}

fn push_token(output: &mut String, token: &str) {
    if !output.is_empty() && !output.ends_with(' ') {
        output.push(' ');
    }
    output.push_str(token);
}

fn push_unique(forms: &mut Vec<String>, form: String) {
    if !form.is_empty() && !forms.contains(&form) {
        forms.push(form);
    }
}


#[cfg(test)]
#[path = "system_settings_search_tests.rs"]
mod tests;
