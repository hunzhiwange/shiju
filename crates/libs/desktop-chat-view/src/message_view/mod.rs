pub mod parse;
#[derive(Debug, Clone)]
/// 描述 ParsedChatBlock 支持的离散状态或消息分支。
pub enum ParsedChatBlock {
    Think { content: String, open: bool },
    Tool { raw: String },
    Text { content: String },
}


#[cfg(test)]
mod parse_tests;
pub mod think;
#[cfg(test)]
mod think_tests;

pub mod contracts;
pub mod permission_card;
pub mod assistant_error;
pub mod assistant_body;
pub mod error_style;
#[cfg(test)]
mod contracts_tests;
#[cfg(test)]
mod assistant_body_tests;
#[cfg(test)]
mod assistant_error_tests;

pub mod layout;
