//! Workflow presentation keeps source templates separate from user and service data.
use super::{I18n, format::Argument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    pub source: String,
    pub arguments: Vec<String>,
    raw: bool,
    fallback: String,
}

impl Feedback {
    pub fn new(source: impl Into<String>, arguments: Vec<String>) -> Self {
        let source = source.into();
        let values = arguments.iter().cloned().map(Argument::from).collect::<Vec<_>>();
        let fallback = I18n::new(Default::default())
            .format(&source, &values)
            .expect("valid workflow source template");
        Self { source, arguments, raw: false, fallback }
    }

    pub fn message(&self) -> super::format::Message {
        if self.raw {
            super::format::Message::new("%s", vec![self.fallback.clone().into()])
        } else {
            super::format::Message::new(
                &self.source,
                self.arguments.iter().cloned().map(Into::into).collect(),
            )
        }
    }

    pub fn as_str(&self) -> &str {
        &self.fallback
    }

    pub fn render(&self, i18n: I18n) -> String {
        if self.raw {
            return self.fallback.clone();
        }
        let arguments = self.arguments.iter().cloned().map(Argument::from).collect::<Vec<_>>();
        i18n.format(&self.source, &arguments).expect("valid workflow feedback template")
    }
}

impl From<String> for Feedback {
    fn from(value: String) -> Self {
        Self { source: value.clone(), arguments: vec![], raw: true, fallback: value }
    }
}
impl From<&str> for Feedback {
    fn from(value: &str) -> Self {
        Self::new(value, vec![])
    }
}
impl From<Feedback> for String {
    fn from(value: Feedback) -> Self {
        value.fallback
    }
}
impl std::ops::Deref for Feedback {
    type Target = str;
    fn deref(&self) -> &str {
        &self.fallback
    }
}
impl std::fmt::Display for Feedback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.fallback)
    }
}
impl PartialEq<&str> for Feedback {
    fn eq(&self, other: &&str) -> bool {
        self.fallback == *other
    }
}

impl std::error::Error for Feedback {}
