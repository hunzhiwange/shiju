//! 云端资料的数字分页、账号边界与可重试正文状态。
use super::LoadState;
use crate::app::account::SecretSkey;
use vw_gateway_client::{
    ShijuKnowledgeBaseDto, ShijuKnowledgeDocumentDto, ShijuKnowledgePageDto,
    ShijuKnowledgePageQuery, ShijuKnowledgeReadDto, ShijuKnowledgeSearchDto,
};

#[derive(Debug, Clone)]
pub(crate) struct KnowledgePage<T> {
    pub(crate) query: ShijuKnowledgePageQuery,
    pub(crate) total: u64,
    pub(crate) items: Vec<T>,
    pub(crate) status: LoadState<()>,
    pending: Option<KnowledgeRequest>,
    corrected: bool,
}
impl<T> Default for KnowledgePage<T> {
    fn default() -> Self {
        Self {
            query: ShijuKnowledgePageQuery::default(),
            total: 0,
            items: vec![],
            status: LoadState::Idle,
            pending: None,
            corrected: false,
        }
    }
}
impl<T> KnowledgePage<T> {
    pub(crate) fn pages(&self) -> u32 {
        let size = u64::from(self.query.page_size.max(1));
        (self.total / size + u64::from(self.total % size != 0)).max(1).min(u64::from(u32::MAX))
            as u32
    }
    fn begin(&mut self, request: KnowledgeRequest) {
        self.status = LoadState::Loading { generation: request.generation };
        self.pending = Some(request);
    }
    /// 返回 true 时只允许补发一次最后有效页。
    fn finish(
        &mut self,
        request: &KnowledgeRequest,
        result: Result<ShijuKnowledgePageDto<T>, String>,
    ) -> Option<bool> {
        if self.pending.as_ref() != Some(request) {
            return None;
        }
        self.pending = None;
        match result {
            Err(error) => self.status = LoadState::Error(error.into()),
            Ok(page) => {
                if page.page != self.query.page || page.page_size != self.query.page_size {
                    self.status = LoadState::Error("invalid_knowledge_response".into());
                    return Some(false);
                }
                self.total = page.total;
                if self.query.page > self.pages() && !self.corrected {
                    self.query.page = self.pages();
                    self.corrected = true;
                    self.items.clear();
                    return Some(true);
                }
                if self.query.page > self.pages() {
                    self.items.clear();
                    self.status = LoadState::Error("knowledge_page_changed_retry".into());
                    return Some(false);
                }
                self.items = page.items;
                self.status = LoadState::Loaded(());
            }
        }
        Some(false)
    }
    pub(crate) fn refresh(&mut self) {
        self.corrected = false;
    }
    pub(crate) fn set_page(&mut self, page: u32) -> bool {
        if page == 0 || page > self.pages() {
            return false;
        }
        self.query.page = page;
        self.corrected = false;
        true
    }
    pub(crate) fn search(&mut self, q: String) {
        self.query.q = q;
        self.query.page = 1;
        self.corrected = false;
        self.items.clear();
        self.pending = None;
        self.status = LoadState::Idle;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KnowledgeWork {
    Bases(ShijuKnowledgePageQuery),
    Documents(String, ShijuKnowledgePageQuery),
    Search(String, String),
    Read { kb: String, doc: String, offset: u64 },
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeRequest {
    pub(crate) account_id: String,
    pub(crate) generation: u64,
    pub(crate) work: KnowledgeWork,
}
impl std::fmt::Debug for KnowledgeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KnowledgeRequest(<redacted>)")
    }
}
#[derive(Clone)]
pub(crate) struct KnowledgeReader {
    pub(crate) kb_zid: String,
    pub(crate) doc_zid: String,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) total_chars: u64,
    pub(crate) next_offset: Option<u64>,
    pub(crate) status: LoadState<()>,
    pending: Option<KnowledgeRequest>,
}
impl std::fmt::Debug for KnowledgeReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KnowledgeReader(<redacted>)")
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct KnowledgeState {
    pub(crate) account_id: Option<String>,
    credential: Option<SecretSkey>,
    generation: u64,
    pub(crate) bases: KnowledgePage<ShijuKnowledgeBaseDto>,
    pub(crate) documents: KnowledgePage<ShijuKnowledgeDocumentDto>,
    pub(crate) selected_base: Option<String>,
    pub(crate) reader: Option<KnowledgeReader>,
    pub(crate) search_query: String,
    pub(crate) search: LoadState<ShijuKnowledgeSearchDto>,
    search_pending: Option<KnowledgeRequest>,
}
impl KnowledgeState {
    pub(crate) fn clear(&mut self, account_id: Option<String>) {
        let generation = self.generation.wrapping_add(1);
        *self = Self { account_id, generation, ..Self::default() };
    }
    pub(crate) fn sync_account(&mut self, id: String, credential: SecretSkey) {
        if self.account_id.as_deref() != Some(&id) || self.credential.as_ref() != Some(&credential)
        {
            self.clear(Some(id));
            self.credential = Some(credential);
        }
    }
    pub(crate) fn account_matches(&self, id: &str, credential: &SecretSkey) -> bool {
        self.account_id.as_deref() == Some(id) && self.credential.as_ref() == Some(credential)
    }
    fn request(&mut self, work: KnowledgeWork) -> Option<KnowledgeRequest> {
        let account_id = self.account_id.clone()?;
        self.credential.as_ref()?;
        self.generation = self.generation.wrapping_add(1);
        Some(KnowledgeRequest { account_id, generation: self.generation, work })
    }
    fn clear_documents(&mut self) {
        self.documents = KnowledgePage::default();
        self.reader = None;
        self.search_query.clear();
        self.search = LoadState::Idle;
        self.search_pending = None;
    }
    pub(crate) fn select_base(&mut self, kb: String) -> bool {
        if !self.bases.items.iter().any(|item| item.kb_zid == kb) {
            return false;
        }
        self.selected_base = Some(kb);
        self.clear_documents();
        true
    }
    pub(crate) fn set_base_page(&mut self, page: u32) -> bool {
        if !self.bases.set_page(page) {
            return false;
        }
        self.bases.items.clear();
        self.selected_base = None;
        self.clear_documents();
        true
    }
    pub(crate) fn search_bases(&mut self, q: String) {
        self.bases.search(q);
        self.selected_base = None;
        self.clear_documents();
    }
    pub(crate) fn search_documents(&mut self, q: String) {
        self.documents.search(q);
        self.reader = None;
    }
    pub(crate) fn begin_bases(&mut self) -> Option<KnowledgeRequest> {
        let request = self.request(KnowledgeWork::Bases(self.bases.query.clone()))?;
        self.bases.begin(request.clone());
        Some(request)
    }
    pub(crate) fn finish_bases(
        &mut self,
        request: &KnowledgeRequest,
        result: Result<ShijuKnowledgePageDto<ShijuKnowledgeBaseDto>, String>,
    ) -> Option<bool> {
        let correction = self.bases.finish(request, result)?;
        if !self.bases.items.iter().any(|item| Some(&item.kb_zid) == self.selected_base.as_ref()) {
            self.selected_base = self.bases.items.first().map(|item| item.kb_zid.clone());
            self.clear_documents();
        }
        Some(correction)
    }
    pub(crate) fn begin_documents(&mut self) -> Option<KnowledgeRequest> {
        let kb = self.selected_base.clone()?;
        let request = self.request(KnowledgeWork::Documents(kb, self.documents.query.clone()))?;
        self.documents.begin(request.clone());
        Some(request)
    }
    pub(crate) fn finish_documents(
        &mut self,
        request: &KnowledgeRequest,
        result: Result<ShijuKnowledgePageDto<ShijuKnowledgeDocumentDto>, String>,
    ) -> Option<bool> {
        let result = result.and_then(|page| {
            if page.items.iter().any(|item| Some(&item.kb_zid) != self.selected_base.as_ref()) {
                Err("invalid_knowledge_response".into())
            } else {
                Ok(page)
            }
        });
        self.documents.finish(request, result)
    }
    pub(crate) fn begin_search(&mut self, query: String) -> Option<KnowledgeRequest> {
        if query.trim().is_empty() {
            return None;
        }
        let request =
            self.request(KnowledgeWork::Search(self.selected_base.clone()?, query.clone()))?;
        self.search_query = query;
        self.search = LoadState::Loading { generation: request.generation };
        self.search_pending = Some(request.clone());
        Some(request)
    }
    pub(crate) fn finish_search(
        &mut self,
        request: &KnowledgeRequest,
        result: Result<ShijuKnowledgeSearchDto, String>,
    ) {
        if self.search_pending.as_ref() != Some(request) {
            return;
        }
        self.search_pending = None;
        self.search = match result {
            Ok(result)
                if result
                    .records
                    .iter()
                    .all(|item| Some(&item.kb_zid) == self.selected_base.as_ref()) =>
            {
                LoadState::Loaded(result)
            }
            Ok(_) => LoadState::Error("invalid_knowledge_response".into()),
            Err(error) => LoadState::Error(error.into()),
        };
    }
    pub(crate) fn open_reader(&mut self, kb: String, doc: String) -> bool {
        let allowed = self
            .documents
            .items
            .iter()
            .any(|item| item.kb_zid == kb && item.doc_zid == doc)
            || matches!(&self.search, LoadState::Loaded(result) if result.records.iter().any(|item| item.kb_zid == kb && item.doc_zid == doc));
        if !allowed || self.selected_base.as_ref() != Some(&kb) {
            return false;
        }
        self.reader = Some(KnowledgeReader {
            kb_zid: kb,
            doc_zid: doc,
            title: String::new(),
            content: String::new(),
            total_chars: 0,
            next_offset: Some(0),
            status: LoadState::Idle,
            pending: None,
        });
        true
    }
    pub(crate) fn close_reader(&mut self) {
        self.reader = None;
    }
    pub(crate) fn begin_read(&mut self) -> Option<KnowledgeRequest> {
        let reader = self.reader.as_ref()?;
        if reader.pending.is_some() {
            return None;
        }
        let work = KnowledgeWork::Read {
            kb: reader.kb_zid.clone(),
            doc: reader.doc_zid.clone(),
            offset: reader.next_offset?,
        };
        let request = self.request(work)?;
        let reader = self.reader.as_mut()?;
        reader.pending = Some(request.clone());
        reader.status = LoadState::Loading { generation: request.generation };
        Some(request)
    }
    pub(crate) fn finish_read(
        &mut self,
        request: &KnowledgeRequest,
        result: Result<ShijuKnowledgeReadDto, String>,
    ) {
        let Some(reader) = self.reader.as_mut() else {
            return;
        };
        if reader.pending.as_ref() != Some(request) {
            return;
        }
        reader.pending = None;
        match result {
            Err(error) => reader.status = LoadState::Error(error.into()),
            Ok(page) => {
                let KnowledgeWork::Read { offset, .. } = request.work else {
                    return;
                };
                if page.kb_zid != reader.kb_zid
                    || page.doc_zid != reader.doc_zid
                    || page
                        .next_offset
                        .is_some_and(|next| next <= offset || next > page.total_chars)
                {
                    reader.status = LoadState::Error("invalid_knowledge_response".into());
                    return;
                }
                reader.content.push_str(&page.content);
                reader.title = page.title;
                reader.total_chars = page.total_chars;
                reader.next_offset = page.next_offset;
                reader.status = LoadState::Loaded(());
            }
        }
    }
}
