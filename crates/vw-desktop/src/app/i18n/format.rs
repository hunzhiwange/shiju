//! Bounded, typed sprintf subset. Arguments are data, never templates.
use super::I18n;

#[derive(Debug, Clone, PartialEq)]
pub enum Argument {
    Text(String),
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}
impl From<&str> for Argument {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<String> for Argument {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
macro_rules! conversions { ($variant:ident: $($ty:ty),*) => { $(impl From<$ty> for Argument { fn from(v: $ty) -> Self { Self::$variant(v as _) } })* }; }
conversions!(Signed: i8, i16, i32, i64, isize);
conversions!(Unsigned: u8, u16, u32, u64, usize);
conversions!(Float: f32, f64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError(pub &'static str);
impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for FormatError {}
type Result<T> = std::result::Result<T, FormatError>;

#[derive(Debug, Clone)]
pub enum Message {
    Template { source: String, arguments: Vec<Argument> },
    Lines(Vec<Message>),
    Literal(String),
    Nested { source: String, arguments: Vec<Message> },
}
impl Message {
    pub fn literal(value: String) -> Self {
        Self::Literal(value)
    }
    pub fn nested(source: String, arguments: Vec<Message>) -> Self {
        Self::Nested { source, arguments }
    }
    pub fn new(source: impl Into<String>, arguments: Vec<Argument>) -> Self {
        Self::Template { source: source.into(), arguments }
    }
    pub fn lines(messages: Vec<Message>) -> Self {
        Self::Lines(messages)
    }
    pub fn render(&self, i18n: I18n) -> Result<String> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Nested { source, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|argument| argument.render(i18n).map(Argument::Text))
                    .collect::<Result<Vec<_>>>()?;
                i18n.format(source, &arguments)
            }
            Self::Template { source, arguments } => i18n.format(source, arguments),
            Self::Lines(messages) => messages
                .iter()
                .map(|message| message.render(i18n))
                .collect::<Result<Vec<_>>>()
                .map(|lines| lines.join("\n")),
        }
    }
}
#[derive(Debug)]
enum Part {
    Text(String),
    Field { index: usize, kind: char, width: usize, precision: Option<usize> },
}
const LIMIT: usize = 4096;
fn number(chars: &[char], cursor: &mut usize) -> Result<Option<usize>> {
    let start = *cursor;
    let mut value = 0usize;
    while let Some(digit) = chars.get(*cursor).and_then(|c| c.to_digit(10)) {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit as usize))
            .filter(|v| *v <= LIMIT)
            .ok_or(FormatError("format limit exceeded"))?;
        *cursor += 1;
    }
    Ok((*cursor > start).then_some(value))
}
fn parse(template: &str) -> Result<(Vec<Part>, Vec<char>)> {
    let chars: Vec<_> = template.chars().collect();
    let (mut cursor, mut next) = (0, 0);
    let mut mode = None;
    let mut parts = Vec::new();
    let mut signature = Vec::new();
    while cursor < chars.len() {
        if chars[cursor] != '%' {
            parts.push(Part::Text(chars[cursor].to_string()));
            cursor += 1;
            continue;
        }
        cursor += 1;
        if chars.get(cursor) == Some(&'%') {
            parts.push(Part::Text("%".into()));
            cursor += 1;
            continue;
        }
        let first = number(&chars, &mut cursor)?;
        let positional = chars.get(cursor) == Some(&'$');
        if mode.is_some_and(|old| old != positional) {
            return Err(FormatError("mixed argument positions"));
        }
        mode = Some(positional);
        let (index, width) = if positional {
            cursor += 1;
            let index = first
                .and_then(|v| v.checked_sub(1))
                .ok_or(FormatError("invalid argument position"))?;
            (index, number(&chars, &mut cursor)?.unwrap_or(0))
        } else {
            let index = next;
            next += 1;
            (index, first.unwrap_or(0))
        };
        let precision = if chars.get(cursor) == Some(&'.') {
            cursor += 1;
            Some(number(&chars, &mut cursor)?.ok_or(FormatError("missing precision"))?)
        } else {
            None
        };
        let kind = *chars.get(cursor).ok_or(FormatError("incomplete directive"))?;
        cursor += 1;
        if !matches!(kind, 's' | 'd' | 'u' | 'f') {
            return Err(FormatError("unsupported directive"));
        }
        if signature.len() <= index {
            signature.resize(index + 1, '\0');
        }
        if signature[index] != '\0' && signature[index] != kind {
            return Err(FormatError("conflicting argument types"));
        }
        signature[index] = kind;
        parts.push(Part::Field { index, kind, width, precision });
    }
    if signature.contains(&'\0') {
        return Err(FormatError("argument position gap"));
    }
    Ok((parts, signature))
}
impl I18n {
    pub fn format(&self, source: &str, arguments: &[Argument]) -> Result<String> {
        let (_, original) = parse(source)?;
        let (parts, translated) = parse(self.__(source))?;
        if original != translated {
            return Err(FormatError("translation signature mismatch"));
        }
        if arguments.len() != original.len() {
            return Err(FormatError("argument count mismatch"));
        }
        let mut output = String::new();
        for part in parts {
            match part {
                Part::Text(value) => output.push_str(&value),
                Part::Field { index, kind, width, precision } => {
                    let value = match (&arguments[index], kind) {
                        (Argument::Text(v), 's') => {
                            v.chars().take(precision.unwrap_or(usize::MAX)).collect()
                        }
                        (Argument::Signed(v), 'd') => integer(v.to_string(), precision),
                        (Argument::Unsigned(v), 'u') => integer(v.to_string(), precision),
                        (Argument::Float(v), 'f') => format!("{:.*}", precision.unwrap_or(6), v),
                        _ => return Err(FormatError("argument type mismatch")),
                    };
                    output.extend(std::iter::repeat_n(
                        ' ',
                        width.saturating_sub(value.chars().count()),
                    ));
                    output.push_str(&value);
                }
            }
        }
        Ok(output)
    }
}
fn integer(value: String, precision: Option<usize>) -> String {
    let negative = value.starts_with('-');
    let digits = value.trim_start_matches('-');
    let digits = if precision == Some(0) && digits == "0" { "" } else { digits };
    format!(
        "{}{}{}",
        if negative { "-" } else { "" },
        "0".repeat(precision.unwrap_or(0).saturating_sub(digits.len())),
        digits
    )
}

#[macro_export]
macro_rules! __ {
    ($i18n:expr, $source:expr $(, $argument:expr)* $(,)?) => {{
        ($i18n).format($source, &[$($crate::app::i18n::format::Argument::from($argument)),*])
    }};
}
