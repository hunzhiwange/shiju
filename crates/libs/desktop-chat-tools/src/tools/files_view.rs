pub mod parse;
#[cfg(test)]
mod parse_tests;

pub struct FileListState {
    pub items_for_display: Vec<(String, String)>,
    pub total_items: usize,
    pub display_count: usize,
    pub truncated_middle: bool,
    pub middle_omitted: usize,
    pub tail_omitted: usize,
    pub filter_query: String,
    pub is_empty_filtered: bool,
    pub max_items: usize,
    pub is_search: bool,
}
