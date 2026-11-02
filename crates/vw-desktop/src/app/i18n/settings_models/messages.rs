//! Locale-independent feedback stored until display.
use vw_config_types::ui::DesktopLocale;
#[path = "message_catalog.rs"]
mod catalog;
pub use catalog::render;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feedback {
    original: String,
    owned: Option<(&'static str, Vec<String>)>,
    joined: Option<Box<(Feedback, Feedback)>>,
}
impl Feedback {
    pub fn owned(key: &'static str, args: Vec<String>) -> Self {
        Self {
            original: render(DesktopLocale::SimplifiedChinese, key, &args),
            owned: Some((key, args)),
            joined: None,
        }
    }
    pub fn join(self, next: Feedback) -> Self {
        let original = format!("{}\n{}", self.original, next.original);
        Self { original, owned: None, joined: Some(Box::new((self, next))) }
    }
    pub fn display(&self, locale: DesktopLocale) -> String {
        if let Some(parts) = &self.joined {
            return format!("{}\n{}", parts.0.display(locale), parts.1.display(locale));
        }
        match &self.owned {
            Some((key, args)) => render(locale, *key, args),
            None => self.original.clone(),
        }
    }
}
impl From<String> for Feedback {
    fn from(original: String) -> Self {
        Self { original, owned: None, joined: None }
    }
}
impl std::ops::Deref for Feedback {
    type Target = str;
    fn deref(&self) -> &str {
        &self.original
    }
}
impl std::fmt::Display for Feedback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.original)
    }
}

impl From<&str> for Feedback {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
#[cfg(test)]
impl Feedback {
    pub fn as_str(&self) -> &str {
        &self.original
    }
}
#[cfg(test)]
impl PartialEq<&str> for Feedback {
    fn eq(&self, other: &&str) -> bool {
        self.original == *other
    }
}

impl Feedback {
    pub fn discovery(reason: &str) -> Self {
        let key = match reason {
            "timeout" => "自动发现模型失败：请求超时",
            "request" => "自动发现模型失败：网络请求失败",
            "http_status" => "自动发现模型失败：服务返回错误状态",
            "redirect" => "自动发现模型失败：服务返回重定向",
            "response_too_large" => "自动发现模型失败：响应过大",
            "invalid_response" => "自动发现模型失败：返回格式无效",
            "client" => "自动发现模型失败：请求初始化失败",
            _ => "自动发现模型失败：未知错误",
        };
        Self::owned(key, vec![])
    }
}

impl Feedback {
    pub fn gateway(error: impl ToString) -> Self {
        Self::owned("服务端配置不可达，请检查 Gateway 连接状态。%s", vec![error.to_string()])
    }
}
