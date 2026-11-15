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

pub mod markdown;
pub mod markdown_code;
pub mod markdown_links;
pub mod markdown_rich;
#[cfg(test)]
mod markdown_tests;

pub mod styles;
pub mod text;
pub mod text_normalize;
#[cfg(test)]
mod styles_tests;
#[cfg(test)]
mod text_tests;

pub mod markdown_selection;
pub mod selection_menu;

pub mod tool_summaries;
pub mod cache_text_support;
pub mod render_cache;

mod cache_tools;
#[cfg(test)]
mod render_cache_tests;
#[cfg(test)]
mod tool_summaries_tests;

pub mod think_block;
#[cfg(test)]
mod think_block_tests;
