//! Locale-independent feedback stored until display.
use vw_config_types::ui::DesktopLocale;
#[path = "message_catalog.rs"]
mod catalog;
pub use catalog::{Message, render};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feedback {
    original: String,
    owned: Option<(Message, Vec<String>)>,
    joined: Option<Box<(Feedback, Feedback)>>,
}
impl Feedback {
    pub fn owned(key: Message, args: Vec<String>) -> Self {
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
            "timeout" => Message::DiscoveryTimeout,
            "request" => Message::DiscoveryRequest,
            "http_status" => Message::DiscoveryHttpStatus,
            "redirect" => Message::DiscoveryRedirect,
            "response_too_large" => Message::DiscoveryTooLarge,
            "invalid_response" => Message::DiscoveryInvalidResponse,
            "client" => Message::DiscoveryClient,
            _ => Message::DiscoveryUnknown,
        };
        Self::owned(key, vec![])
    }
}

impl Feedback {
    pub fn gateway(error: impl ToString) -> Self {
        Self::owned(Message::GatewayConfigUnavailable, vec![error.to_string()])
    }
}
