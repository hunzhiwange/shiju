pub use desktop_chat_view::render_cache::RevisionRenderKey;
pub(crate) fn build_render_cache_entry(raw: &str, visible: &str, hash: u64, reasoning: bool) -> crate::app::models::ChatRenderCacheEntry {
    #[cfg(test)] {
        super::super::performance::record_full_content_parse(raw.len());
        super::super::performance::record_full_content_hash(raw.len());
    }
    desktop_chat_view::message_view::render_cache::build_render_cache_entry(raw, visible, hash, reasoning)
}
pub(super) fn resolve_visible_text_and_copy_hash<'a>(app: &'a crate::app::App, idx: usize, content: &'a str, hash: u64, cache: &crate::app::models::ChatRenderCacheEntry) -> (std::borrow::Cow<'a, str>, u64) {
    #[cfg(test)]
    if cache.content_hash != hash || app.chat_visible_text_cache.get(idx).and_then(|t| t.as_deref()).is_none() || app.chat_copy_hash_cache.get(idx).and_then(|h| *h).is_none() {
        let (_, visible, _) = desktop_chat_view::message_view::think::split_think(content);
        super::super::performance::record_full_content_hash(visible.len());
    }
    desktop_chat_view::message_view::render_cache::resolve_visible_text_and_copy_hash(app.chat_visible_text_cache.get(idx).and_then(|t| t.as_deref()), app.chat_copy_hash_cache.get(idx).and_then(|h| *h), content, hash, cache)
}

pub(crate) fn effective_assistant_render_cache<'a>(content: &str, hash: u64, cache: &'a crate::app::models::ChatRenderCacheEntry, visible: &str, copy_hash: u64, streaming: bool, reasoning: bool) -> std::borrow::Cow<'a, crate::app::models::ChatRenderCacheEntry> {
    #[cfg(test)]
    if cache.content_hash != hash || cache.show_reasoning_summary != reasoning {
        super::super::performance::record_full_content_parse(content.len());
        super::super::performance::record_full_content_hash(content.len());
    }
    desktop_chat_view::message_view::render_cache::effective_assistant_render_cache(content, hash, cache, visible, copy_hash, streaming, reasoning)
}
pub(crate) fn assistant_render_blocks<'a>(content: &str, hash: u64, cache: &'a crate::app::models::ChatRenderCacheEntry, streaming: bool) -> (std::borrow::Cow<'a, [crate::app::models::ParsedChatBlock]>, bool) {
    #[cfg(test)]
    if cache.content_hash != hash { super::super::performance::record_full_content_parse(content.len()); }
    desktop_chat_view::message_view::render_cache::assistant_render_blocks(content, hash, cache, streaming)
}
